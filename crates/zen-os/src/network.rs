use std::net::IpAddr;

#[derive(Debug, Clone)]
pub struct DnsRule {
    pub domain: String,
    pub action: DnsAction,
}

#[derive(Debug, Clone)]
pub enum DnsAction {
    Block,
    Redirect { ip: IpAddr },
}
