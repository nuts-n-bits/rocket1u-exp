type UrlComponentDecoder<TDecoderError> = fn (s: &str) -> Result<String, TDecoderError>;

#[derive(Debug)]
pub struct ParsedUrl<'a> {
    before_query: &'a str,
    after_query: Option<&'a str>,
    decoded_segments: Vec<String>,
    decoded_queries: Vec<(String, Option<String>)>,
}

#[derive(Debug)]
pub struct ParsedUrlWithQueryMap<'a> {
    pu: ParsedUrl<'a>,
    qm: std::collections::HashMap<&'a str, String>,
}

#[derive(Debug)]
pub enum DuplicateParamPolicy<'a> {
    ResultErr,
    KeepFirst,
    KeepLast,
    ConcatWithDelim(&'a str)
}

#[derive(Debug)]
pub struct DupParamError(String);

impl<'a> ParsedUrl<'a> {
    pub fn decoded_query_map(self: &'a Self, null_replacement: &'a str, dup_policy: DuplicateParamPolicy) -> Result<std::collections::HashMap<&'a str, String>, DupParamError> {
        let mut hashmap = std::collections::HashMap::<&'a str, String>::new();
        for (k, v) in &self.decoded_queries {
            if let Some(existing_value) = hashmap.get::<str>(k) {
                match dup_policy {
                    DuplicateParamPolicy::ResultErr => { return Err(DupParamError(k.clone())) }
                    DuplicateParamPolicy::KeepFirst => {}
                    DuplicateParamPolicy::KeepLast => { hashmap.insert(&k, String::from(v.as_deref().unwrap_or(null_replacement))); }
                    DuplicateParamPolicy::ConcatWithDelim(delim) => { 
                        let concat = concat(&existing_value, delim, &v.as_deref().unwrap_or(null_replacement));
                        hashmap.insert(&k, concat);
                    }
                }
            }
            else {
                hashmap.insert(&k, v.clone().unwrap_or(String::from(null_replacement)));
            }
        }
        Ok(hashmap)
    }

    pub fn parse_new<TDecoderError>(raw_url: &str, decoder: UrlComponentDecoder<TDecoderError>) -> Result<ParsedUrl, TDecoderError> {
        let (before_query, after_query) = split_at_first_delim(raw_url, "?");
        let decoded_segments = (if before_query.len() == 0 { "" } else { &before_query[1..] }).split("/").map(decoder).collect::<Result<Vec<String>, TDecoderError>>()?;
        let decoded_queries = {
            if let Some(after_query_concrete) = after_query { after_query_concrete.split("&").map(|query_entry| query_decoder(query_entry, decoder)).collect::<Result<Vec<(String, Option<String>)>, TDecoderError>>()? }
            else { Vec::new() }
        };
        return Ok(ParsedUrl {
            before_query, 
            after_query, 
            decoded_segments,
            decoded_queries
        })
    }
}

fn query_decoder<TDecoderError>(input: &str, decoder: UrlComponentDecoder<TDecoderError>) -> Result<(String, Option<String>), TDecoderError> {
    let (pre_before, pre_after) = split_at_first_delim(input, "=");
    let before = decoder(pre_before)?;
    let after = if let Some(after) = pre_after { Some(decoder(after)?) } else { None };
    return Ok((before, after))
}

fn concat(str1: &str, str2: &str, str3: &str) -> String {
    let mut owned_string = String::from(str1);
    owned_string.push_str(str2);
    owned_string.push_str(str3);
    return owned_string;
}

fn split_at_first_delim<'a>(s: &'a str, delim: &'a str) -> (&'a str, Option<&'a str>) {
    let pos = s.find(delim);
    if let Some(pos) = pos { (&s[..pos], Some(&s[pos+delim.len()..])) }
    else { (s, None) } 
}

mod test {

    use rouille::url::form_urlencoded::Parse;

    use super::*;

    fn s(str: &str) -> String { String::from(str) }
    fn identity_decoder(s: &str) -> Result<String, ()> { Ok(String::from(s)) }  // TODO: use never type to replace unit type when its available

    #[test]
    fn test() {
        let pu01 = ParsedUrl::parse_new("/lol", identity_decoder).unwrap();
        assert_eq!(pu01.before_query, "/lol");
        assert_eq!(pu01.after_query, None);
        assert_eq!(pu01.decoded_segments.len(), 1);
        assert_eq!(pu01.decoded_segments[0], "lol");
        assert_eq!(pu01.decoded_queries.len(), 0);

        let pu02 = ParsedUrl::parse_new("", identity_decoder).unwrap();
        assert_eq!(pu02.before_query, "");
        assert_eq!(pu02.after_query, None);
        assert_eq!(pu02.decoded_segments.len(), 1);
        assert_eq!(pu02.decoded_segments[0], "");
        assert_eq!(pu02.decoded_queries.len(), 0);

        let pu03 = ParsedUrl::parse_new("/api/pbv4/some-endpoint?arg1=val1&arg2=val2&arg3=val3", identity_decoder).unwrap();
        let qm03 = pu03.decoded_query_map("", DuplicateParamPolicy::KeepFirst).unwrap();
        assert_eq!(pu03.before_query, "/api/pbv4/some-endpoint");
        assert_eq!(pu03.after_query, Some("arg1=val1&arg2=val2&arg3=val3"));
        assert_eq!(pu03.decoded_segments.len(), 3);
        assert_eq!(pu03.decoded_segments[0], "api");
        assert_eq!(pu03.decoded_segments[1], "pbv4");
        assert_eq!(pu03.decoded_segments[2], "some-endpoint");
        assert_eq!(pu03.decoded_queries.len(), 3);
        assert_eq!(pu03.decoded_queries[0], (s("arg1"), Some(s("val1"))));
        assert_eq!(pu03.decoded_queries[1], (s("arg2"), Some(s("val2"))));
        assert_eq!(pu03.decoded_queries[2], (s("arg3"), Some(s("val3"))));
        assert_eq!(qm03.get("arg1"), Some(&s("val1")));
        assert_eq!(qm03.get("arg2"), Some(&s("val2")));
        assert_eq!(qm03.get("arg3"), Some(&s("val3")));
        assert_eq!(qm03.get("arg4"), None);

        let pu04 = ParsedUrl::parse_new("/?arg1&arg2", identity_decoder).unwrap();
        let qm04 = pu04.decoded_query_map("", DuplicateParamPolicy::KeepFirst).unwrap();
        assert_eq!(qm04.get("arg1"), Some(&s("")));
        assert_eq!(qm04.get("arg2"), Some(&s("")));
        assert_eq!(qm04.get("arg3"), None);

        let pu05 = ParsedUrl::parse_new("/path?id=1&id=2", identity_decoder).unwrap();
        let qm05 = pu05.decoded_query_map("", DuplicateParamPolicy::KeepFirst).unwrap();
        assert_eq!(qm05.get("id"), Some(&s("1")));
        let qm05 = pu05.decoded_query_map("", DuplicateParamPolicy::KeepLast).unwrap();
        assert_eq!(qm05.get("id"), Some(&s("2")));
        let qm05 = pu05.decoded_query_map("", DuplicateParamPolicy::ResultErr);
        assert!(qm05.is_err());
        let qm05 = pu05.decoded_query_map("", DuplicateParamPolicy::ConcatWithDelim("")).unwrap();
        assert_eq!(qm05.get("id"), Some(&s("12")));
        let qm05 = pu05.decoded_query_map("", DuplicateParamPolicy::ConcatWithDelim(" ")).unwrap();
        assert_eq!(qm05.get("id"), Some(&s("1 2")));

        let pu06 = ParsedUrl::parse_new("/path?noval1&noval2&noval2&noval2", identity_decoder).unwrap();
        let qm06 = pu06.decoded_query_map("null", DuplicateParamPolicy::ConcatWithDelim(" ")).unwrap();
        assert_eq!(qm06.get("noval1"), Some(&s("null")));
        assert_eq!(qm06.get("noval2"), Some(&s("null null null")));
    }

    #[test]
    fn test_split_at_first_delim() {
        let (before, after) = split_at_first_delim("a=b", "=");
        assert_eq!((before, after), ("a", Some("b")));
        
        let (before, after) = split_at_first_delim("a=", "=");
        assert_eq!((before, after), ("a", Some("")));
        
        let (before, after) = split_at_first_delim("a", "=");
        assert_eq!((before, after), ("a", None));
        
        let (before, after) = split_at_first_delim("", "=");
        assert_eq!((before, after), ("", None));
        
        let (before, after) = split_at_first_delim("=====", "=");
        assert_eq!((before, after), ("", Some("====")));
    }
}