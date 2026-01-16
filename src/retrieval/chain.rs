use super::backend::{RetrievedTexture, RetrievedTextureBytes, TextureRetriever};
use crate::models::TextureType;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

/// Chain of texture retrievers that tries each handler in order
/// Returns the first successfully retrieved texture
pub struct ChainRetriever {
    handlers: Vec<Arc<dyn TextureRetriever>>,
}

impl ChainRetriever {
    /// Create a new chain with the given handlers
    /// Handlers are tried in the order they are provided
    pub fn new(handlers: Vec<Arc<dyn TextureRetriever>>) -> Self {
        ChainRetriever { handlers }
    }

    /// Add a handler to the end of the chain
    pub fn add_handler(mut self, handler: Arc<dyn TextureRetriever>) -> Self {
        self.handlers.push(handler);
        self
    }

    /// Get the number of handlers in the chain
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    /// Check if the chain is empty
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}

#[async_trait]
impl TextureRetriever for ChainRetriever {
    async fn get_texture(
        &self,
        user_uuid: Uuid,
        texture_type: TextureType,
    ) -> Result<Option<RetrievedTexture>> {
        // Try each handler in order
        for (index, handler) in self.handlers.iter().enumerate() {
            // Skip handlers that don't support this texture type
            if !handler.supports_texture_type(texture_type) {
                tracing::debug!(
                    "Handler {} does not support texture type {:?}, skipping",
                    index,
                    texture_type
                );
                continue;
            }

            tracing::debug!(
                "Trying handler {} for texture type {:?}",
                index,
                texture_type
            );

            match handler.get_texture(user_uuid, texture_type).await {
                Ok(Some(texture)) => {
                    tracing::debug!(
                        "Handler {} successfully retrieved texture for user {}",
                        index,
                        user_uuid
                    );
                    return Ok(Some(texture));
                }
                Ok(None) => {
                    tracing::debug!(
                        "Handler {} found no texture for user {}, trying next handler",
                        index,
                        user_uuid
                    );
                    // Continue to next handler
                }
                Err(e) => {
                    tracing::warn!(
                        "Handler {} failed with error: {}, trying next handler",
                        index,
                        e
                    );
                    // Continue to next handler on error
                }
            }
        }

        tracing::debug!(
            "No handler in the chain could retrieve texture type {:?} for user {}",
            texture_type,
            user_uuid
        );

        Ok(None)
    }

    async fn get_texture_bytes(
        &self,
        user_uuid: Uuid,
        texture_type: TextureType,
    ) -> Result<Option<RetrievedTextureBytes>> {
        // Try each handler in order
        for (index, handler) in self.handlers.iter().enumerate() {
            // Skip handlers that don't support this texture type
            if !handler.supports_texture_type(texture_type) {
                continue;
            }

            match handler.get_texture_bytes(user_uuid, texture_type).await {
                Ok(Some(texture_bytes)) => {
                    tracing::debug!(
                        "Handler {} successfully retrieved texture bytes for user {}",
                        index,
                        user_uuid
                    );
                    return Ok(Some(texture_bytes));
                }
                Ok(None) => {
                    // Continue to next handler
                }
                Err(e) => {
                    tracing::warn!(
                        "Handler {} failed with error: {}, trying next handler",
                        index,
                        e
                    );
                    // Continue to next handler on error
                }
            }
        }

        Ok(None)
    }

    async fn get_texture_bytes_by_hash(
        &self,
        hash: &str,
    ) -> Result<Option<RetrievedTextureBytes>> {
        // Try each handler in order
        for (index, handler) in self.handlers.iter().enumerate() {
            match handler.get_texture_bytes_by_hash(hash).await {
                Ok(Some(texture_bytes)) => {
                    tracing::debug!(
                        "Handler {} successfully retrieved texture bytes for hash {}",
                        index,
                        hash
                    );
                    return Ok(Some(texture_bytes));
                }
                Ok(None) => {
                    // Continue to next handler
                }
                Err(e) => {
                    tracing::warn!(
                        "Handler {} failed with error: {}, trying next handler",
                        index,
                        e
                    );
                    // Continue to next handler on error
                }
            }
        }

        Ok(None)
    }

    fn supports_texture_type(&self, texture_type: TextureType) -> bool {
        // Chain supports a texture type if any handler supports it
        self.handlers
            .iter()
            .any(|handler| handler.supports_texture_type(texture_type))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // Mock retriever for testing
    struct MockRetriever {
        name: String,
        supported_types: Vec<TextureType>,
        should_return: Option<RetrievedTexture>,
        should_fail: bool,
    }

    #[async_trait]
    impl TextureRetriever for MockRetriever {
        async fn get_texture(
            &self,
            _user_uuid: Uuid,
            texture_type: TextureType,
        ) -> Result<Option<RetrievedTexture>> {
            if self.should_fail {
                return Err(anyhow::anyhow!("Mock failure"));
            }

            if self.supported_types.contains(&texture_type) {
                Ok(self.should_return.clone())
            } else {
                Ok(None)
            }
        }

        async fn get_texture_bytes(
            &self,
            _user_uuid: Uuid,
            _texture_type: TextureType,
        ) -> Result<Option<RetrievedTextureBytes>> {
            if self.should_fail {
                return Err(anyhow::anyhow!("Mock failure"));
            }
            Ok(None)
        }

        fn supports_texture_type(&self, texture_type: TextureType) -> bool {
            self.supported_types.contains(&texture_type)
        }
    }

    #[tokio::test]
    async fn test_chain_returns_first_successful() {
        let handler1 = Arc::new(MockRetriever {
            name: "handler1".to_string(),
            supported_types: vec![TextureType::SKIN],
            should_return: None,
            should_fail: false,
        });

        let handler2 = Arc::new(MockRetriever {
            name: "handler2".to_string(),
            supported_types: vec![TextureType::SKIN],
            should_return: Some(RetrievedTexture {
                url: "http://example.com/skin.png".to_string(),
                hash: "abc123".to_string(),
                metadata: None,
            }),
            should_fail: false,
        });

        let chain = ChainRetriever::new(vec![handler1, handler2]);

        let result = chain
            .get_texture(Uuid::new_v4(), TextureType::SKIN)
            .await
            .unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().url, "http://example.com/skin.png");
    }

    #[tokio::test]
    async fn test_chain_continues_on_error() {
        let handler1 = Arc::new(MockRetriever {
            name: "handler1".to_string(),
            supported_types: vec![TextureType::SKIN],
            should_return: None,
            should_fail: true, // This handler will fail
        });

        let handler2 = Arc::new(MockRetriever {
            name: "handler2".to_string(),
            supported_types: vec![TextureType::SKIN],
            should_return: Some(RetrievedTexture {
                url: "http://example.com/skin.png".to_string(),
                hash: "abc123".to_string(),
                metadata: None,
            }),
            should_fail: false,
        });

        let chain = ChainRetriever::new(vec![handler1, handler2]);

        let result = chain
            .get_texture(Uuid::new_v4(), TextureType::SKIN)
            .await
            .unwrap();

        // Should still get result from handler2 even though handler1 failed
        assert!(result.is_some());
        assert_eq!(result.unwrap().url, "http://example.com/skin.png");
    }

    #[tokio::test]
    async fn test_chain_returns_none_if_all_fail() {
        let handler1 = Arc::new(MockRetriever {
            name: "handler1".to_string(),
            supported_types: vec![TextureType::SKIN],
            should_return: None,
            should_fail: false,
        });

        let handler2 = Arc::new(MockRetriever {
            name: "handler2".to_string(),
            supported_types: vec![TextureType::SKIN],
            should_return: None,
            should_fail: false,
        });

        let chain = ChainRetriever::new(vec![handler1, handler2]);

        let result = chain
            .get_texture(Uuid::new_v4(), TextureType::SKIN)
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_chain_skips_unsupported_types() {
        let handler1 = Arc::new(MockRetriever {
            name: "handler1".to_string(),
            supported_types: vec![TextureType::CAPE], // Supports CAPE only
            should_return: None,
            should_fail: false,
        });

        let handler2 = Arc::new(MockRetriever {
            name: "handler2".to_string(),
            supported_types: vec![TextureType::SKIN], // Supports SKIN only
            should_return: Some(RetrievedTexture {
                url: "http://example.com/skin.png".to_string(),
                hash: "abc123".to_string(),
                metadata: None,
            }),
            should_fail: false,
        });

        let chain = ChainRetriever::new(vec![handler1, handler2]);

        // Request SKIN - handler1 should be skipped, handler2 should return result
        let result = chain
            .get_texture(Uuid::new_v4(), TextureType::SKIN)
            .await
            .unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().url, "http://example.com/skin.png");
    }
}
