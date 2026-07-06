use async_trait::async_trait;
use std::{collections::HashMap, io::Result};

#[async_trait]
pub trait Services {
    async fn set_client_info<T>(&self, request: tonic::Request<T>) -> Result<()>
    where
        T: Send;

    async fn get_client_info<T>(&self, request: &tonic::Request<T>) -> Result<(String, String)>
    where
        T: Send + std::marker::Sync;
}

#[derive(Default, Debug)]
pub struct CommonClient {
    pub conn_client: HashMap<String, String>,
}

impl CommonClient {
    pub fn set_client(&mut self, ip: String, hostname: String) -> Result<()> {
        self.conn_client.insert(ip, hostname);
        Ok(())
    }

    pub fn remove_client(&mut self, ip: String) -> Result<()> {
        self.conn_client.remove(&ip);
        Ok(())
    }
}
