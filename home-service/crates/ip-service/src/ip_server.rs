use common::services::{self, CommonClient, Services};
use service_protos::proto_ip_service::{
    ip_server::Ip, AddRequest, AddResponse, ListRequest, ListResponse, NetDevice,
};
use tokio::{io::AsyncWriteExt, sync::Mutex};
use tonic::{Request, Response, Result, Status};

use std::{
    collections::HashMap,
    io::{self, Error, ErrorKind},
    net::ToSocketAddrs,
    path::Path,
};

const REMOTE_IP_DIR: &str = "/share/samba/tmp";

#[derive(Default, Debug)]
pub struct IpServer {
    pub conn_clients: Mutex<CommonClient>,
    pub remotes: Mutex<HashMap<String, Vec<NetDevice>>>,
}

impl IpServer {
    async fn sync_remote_ip_to_disk(&self, client_ip: String) -> io::Result<()> {
        let remote_ip_dir = Path::new(REMOTE_IP_DIR);
        if let Err(e) = tokio::fs::create_dir(remote_ip_dir).await {
            if e.kind() != ErrorKind::AlreadyExists {
                return Err(e);
            }
        }
        let file_path = remote_ip_dir.join(client_ip.clone());
        let mut f = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(file_path)
            .await?;
        match self.remotes.lock().await.get(&client_ip) {
            Some(remote_ip) => match serde_json::to_string(remote_ip) {
                Ok(src) => {
                    f.write_all(src.as_bytes()).await.unwrap();
                    Ok(())
                }
                Err(e) => Err(io::Error::new(ErrorKind::InvalidInput, e.to_string())),
            },
            None => Err(Error::new(
                ErrorKind::Other,
                client_ip + " not in remote_addr",
            )),
        }
    }
}

#[async_trait::async_trait]
impl services::Services for IpServer {
    async fn set_client_info<T>(&self, request: Request<T>) -> io::Result<()>
    where
        T: Send,
    {
        let addr = request.remote_addr().ok_or(Error::new(
            ErrorKind::AddrNotAvailable,
            "remote_addr is None",
        ))?;
        self.conn_clients
            .lock()
            .await
            .set_client(addr.ip().to_string(), String::new())?;
        match addr
            .to_socket_addrs()
            .and_then(|mut addrs| addrs.next().ok_or(ErrorKind::AddrNotAvailable.into()))
        {
            Ok(hostname) => {
                self.conn_clients
                    .lock()
                    .await
                    .set_client(addr.ip().to_string(), hostname.to_string())?;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    async fn get_client_info<T>(&self, request: &Request<T>) -> io::Result<(String, String)>
    where
        T: Send + std::marker::Sync,
    {
        match request.remote_addr() {
            Some(addr) => {
                self.conn_clients
                    .lock()
                    .await
                    .set_client(addr.ip().to_string(), String::new())?;
                if let Ok(hostname) = addr
                    .to_socket_addrs()
                    .and_then(|mut addrs| addrs.next().ok_or(ErrorKind::AddrNotAvailable.into()))
                {
                    Ok((addr.ip().to_string(), hostname.to_string()))
                } else {
                    Ok((addr.ip().to_string(), String::new()))
                }
            }
            None => Err(Error::new(
                ErrorKind::AddrNotAvailable,
                "remote_addr is None",
            )),
        }
    }
}

#[async_trait::async_trait]
impl Ip for IpServer {
    async fn list(&self, request: Request<ListRequest>) -> Result<Response<ListResponse>, Status> {
        println!("list request = {:?}", request);
        self.set_client_info(request).await?;
        let response = Response::new(ListResponse::default());
        println!("IpServer.conn_client = {:?}", self.conn_clients);
        println!("list response = {:?}", response);
        Ok(response)
    }

    async fn add_remote(
        &self,
        mut request: Request<AddRequest>,
    ) -> Result<Response<AddResponse>, Status> {
        println!("add_remote request = {:?}", request);
        let remote_net_devices = request.get_mut().clone().net_devices;
        let (client_ip, _) = self.get_client_info(&request).await?;
        self.remotes
            .lock()
            .await
            .insert(client_ip.clone(), remote_net_devices);
        self.set_client_info(request).await?;
        self.sync_remote_ip_to_disk(client_ip).await?;
        let response = Response::new(AddResponse::default());
        println!("IpServer = {:?}", self);
        println!("add_remote response = {:?}", response);
        Ok(response)
    }
}
