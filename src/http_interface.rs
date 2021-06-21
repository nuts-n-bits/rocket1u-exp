struct HttpRequest {
    method: &str,
    url: &str, 
    version: HttpVersion,
    headers: &[(&str, &str)]
}

struct HttpVersion {
    raw: &str,
    major: Option<&str>,
    minor: Option<&str>
}

