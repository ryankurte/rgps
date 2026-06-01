

pub struct Http {

}

impl Http {
    pub async fn bind(bind_addr: SocketAddr) -> Result<Self, std::io::Error> {
        let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    }
}

