use crate::classifier::{Findings, contains_any};
use crate::manifest::Category;

pub fn classify(path: &str, text: &str, findings: &mut Findings) {
    classify_path(path, findings);
    classify_content(text, findings);
}

fn classify_path(path: &str, findings: &mut Findings) {
    if contains_any(
        path,
        &[
            "mount_hosts",
            "host.sh",
            "_hosts",
            "hosts_",
            "/host/",
            "/hosts",
            "hosts/",
        ],
    ) {
        findings.add(
            Category::Hosts,
            "path_hosts",
            "path matches forbidden upstream scope",
        );
    }
    if contains_any(path, &["private_dns", "dns", "adguardhome", "adguard-home"]) {
        findings.add(
            Category::Dns,
            "path_dns",
            "path matches forbidden upstream scope",
        );
    }
    if contains_any(path, &["domain", "域名"]) {
        findings.add(
            Category::Domain,
            "path_domain",
            "path matches forbidden upstream scope",
        );
    }
    if contains_any(path, &["proxy", "proxyconfig", "clash", "mihomo"]) {
        findings.add(
            Category::Proxy,
            "path_proxy",
            "path matches forbidden upstream scope",
        );
    }
    if contains_any(path, &["iptables", "network_limit", "/ip.sh"]) {
        findings.add(
            Category::IptablesNetwork,
            "path_network_blocking",
            "path matches forbidden upstream scope",
        );
    }
    if contains_any(path, &["ad_reward", "广告奖励"]) {
        findings.add(
            Category::AdRewardDomain,
            "path_ad_reward",
            "path matches forbidden upstream scope",
        );
    }
    if path.contains("ifw") {
        findings.add(
            Category::IfwClear,
            "path_ifw",
            "path matches forbidden upstream scope",
        );
    }
}

fn classify_content(text: &str, findings: &mut Findings) {
    if contains_any(
        text,
        &["mount_hosts", "system/etc/hosts", "127.0.0.1 ", "0.0.0.0 "],
    ) {
        findings.add(
            Category::Hosts,
            "content_hosts",
            "content contains forbidden upstream scope",
        );
    }
    if contains_any(
        text,
        &["private_dns", "dns", "adguardhome", "dnsmasq", ":53"],
    ) {
        findings.add(
            Category::Dns,
            "content_dns",
            "content contains forbidden upstream scope",
        );
    }
    if contains_any(
        text,
        &[
            "domain",
            "域名",
            "doubleclick",
            "gdt.qq.com",
            "adservice",
            "admob",
            "googleads",
        ],
    ) {
        findings.add(
            Category::Domain,
            "content_domain",
            "content contains forbidden upstream scope",
        );
    }
    if contains_any(text, &["proxyconfig", "proxy", "clash", "mihomo"]) {
        findings.add(
            Category::Proxy,
            "content_proxy",
            "content contains forbidden upstream scope",
        );
    }
    if contains_any(
        text,
        &["iptables", "redirect", "tproxy", "ip rule", "network_limit"],
    ) {
        findings.add(
            Category::IptablesNetwork,
            "content_network_blocking",
            "content contains forbidden upstream scope",
        );
    }
    if contains_any(text, &["ad_reward", "广告奖励"]) {
        findings.add(
            Category::AdRewardDomain,
            "content_ad_reward",
            "content contains forbidden upstream scope",
        );
    }
    if contains_any(text, &["/data/system/ifw", "ifw规则", "ifw"]) {
        findings.add(
            Category::IfwClear,
            "content_ifw",
            "content contains forbidden upstream scope",
        );
    }
}
