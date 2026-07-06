use tonic::{transport::Channel, Response, Status};

use service_protos::proto_ip_service::{
    ip_client::IpClient, AddRequest, ListRequest, ListResponse, NetDevice,
};

#[derive(Default, Debug, Clone)]
pub struct Client {
    pub server_ip: String,
    pub port: String,
    pub client: Option<IpClient<Channel>>,
}

impl Client {
    pub async fn new(server_ip: String, port: String) -> Self {
        Client {
            server_ip: server_ip.clone(),
            port: port.clone(),
            client: IpClient::connect(
                "http://".to_string() + server_ip.as_str() + ":" + port.as_str(),
            )
            .await
            .ok(),
        }
    }

    pub async fn list(&mut self) -> Result<Response<ListResponse>, Status> {
        println!("list request to server");
        let client = self.client.as_mut().ok_or(std::io::Error::new(
            std::io::ErrorKind::AddrNotAvailable,
            "client is None",
        ))?;
        client.list(ListRequest::default()).await
    }

    pub async fn add_remote(&mut self, net_devices: Vec<NetDevice>) -> Result<(), Status> {
        let request = AddRequest { net_devices };
        if let Some(client) = self.client.as_mut() {
            match client.add_remote(request).await {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            }
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::AddrNotAvailable, "clien is None").into())
        }
    }

    pub async fn re_connect(&mut self) -> Result<(), Status> {
        self.client = IpClient::connect(
            "http://".to_string() + self.server_ip.as_str() + ":" + self.port.as_str(),
        )
        .await
        .ok();
        if self.client.is_none() {
            Err(std::io::Error::new(std::io::ErrorKind::AddrNotAvailable, "clien is None").into())
        } else {
            Ok(())
        }
    }
}
