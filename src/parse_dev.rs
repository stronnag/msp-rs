use url::Url;
use regex::Regex;

pub fn parse_uri_dev (arg: &str) -> (String, u32, u8) {
    let mut name: String ;
    let mut param: u32 = 0;
    let mut typ: u8 = 0;
    let mut need_split = false;
    match Url::parse(arg) {
        Ok(u) => {
            match u.host_str() {
                Some(n) => {
                    name = n.to_string();
		    typ = match u.scheme() {
			"tcp" => 1,
			"udp" => 2,
			_ => 0,
		    };
                    match u.port() {
                        Some(d) => param = u32::from(d),
                        None => param = 5760
                        }
                }
                None => {
                    name = u.path().to_string();
                    need_split = true;
                }
            }
        },
        Err(_) => {
            name = arg.to_string();
            need_split = true;
        }
    };

    if need_split {
        let re = Regex::new(r"[:@]").unwrap();
        let rname = name.clone();
        let mut parts = re.split(&rname);
        match parts.next() {
            Some(n) => name = n.to_string(),
            None => (),
        };
        match parts.next() {
            Some(d) => param = d.parse::<u32>().unwrap(),
            None => param = 115200,
        };
    }
    (name,param,typ)
}
