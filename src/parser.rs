use std::net::IpAddr;

use std::net::SocketAddr;
use url::{Host, Url};


use trust_dns_resolver::Resolver;
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};

fn resolve_domain(domain: &str) -> Result<Vec<IpAddr>, Box<dyn std::error::Error>> {
    // 使用系统默认配置创建解析器
    let resolver = Resolver::new(ResolverConfig::default(), ResolverOpts::default())?;
    let response = resolver.lookup_ip(domain)?;
    let ips: Vec<IpAddr> = response.iter().collect();
    Ok(ips)
}

// url_str -> [ip:port, ...]
pub fn to_adders(url: &Url) -> Result<Vec<SocketAddr>, Box<dyn std::error::Error>> {
    let mut res = vec![];

    match url.host() {
        Some(Host::Domain(domain)) => {
            // dns 解析
            match resolve_domain(domain) {
                Ok(ips) => {
                    assert!(!ips.is_empty(), "Should resolve to at least one IP");

                    let port;
                    if let Some(p) = url.port() {
                        port = p;
                    } else {
                        if url.scheme() == "http"{
                        port = 80;
                        } else if url.scheme() == "https"
                        {
                            port = 443;
                        }else
                        {
                            return Err(format!("unsupported scheme").into());
                        }
                    }
                    for ipi in &ips {
                        res.push(SocketAddr::new(*ipi, port));
                    }
                    return Ok(res);
                }
                Err(e) => return Err(format!("Failed to resolve domain: {}", e).into()),
            }
        }
        Some(Host::Ipv6(_)) => return Err("not support ipv6".into()),
        // 明文ip
        Some(Host::Ipv4(ips)) => {
            let port;
            if let Some(p) = url.port() {
                port = p;
            } else {
                port = 80;
            }
            res.push(SocketAddr::new(IpAddr::V4(ips), port));
            return Ok(res);
        }
        None => return Err("miss host name".into()),
    }
}

