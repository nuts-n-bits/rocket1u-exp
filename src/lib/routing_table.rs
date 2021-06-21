#![allow(incomplete_features)]
//#![feature(unsized_locals, unsized_fn_params)]

use std::collections::HashMap;
use std::fmt::Debug;
use std::iter::FromIterator;

#[derive(Debug)]
pub struct RoutingTable<'a, T: Debug> {
    map: HashMap<&'a str, RoutingTable<'a, T>>,
    data: &'a T,
    depth: usize,
}

#[derive(Debug)]
pub struct RTLookupResult<'a, T: Debug> {
    val: &'a T,
    depth: usize, 
    keys_used: usize,
    keep_going: &'a RoutingTable<'a, T>,
}

#[derive(Copy, Clone)]
pub enum OneOrMore<'a> { One(&'a str), More(&'a [&'a str]) }
pub fn one(str: &str) -> OneOrMore { OneOrMore::One(str) }
pub fn more<'b>(str_arr: &'b [&str]) -> OneOrMore<'b> { OneOrMore::More(&str_arr) }

#[derive(Copy, Clone)]
pub enum SerialOrParallel<'a> { Serial(&'a [&'a str]), Parallel(&'a [&'a str]) }
pub fn ser<'b>(str_arr: &'b [&str]) -> SerialOrParallel<'b> { SerialOrParallel::Serial(str_arr) }
pub fn par<'b>(str_arr: &'b [&str]) -> SerialOrParallel<'b> { SerialOrParallel::Parallel(str_arr) }

impl<'a, T: Debug> RoutingTable<'a, T> {

    pub fn new(root_data:&'a T) -> Self {
        RoutingTable::new_core(root_data, 0)
    }

    fn new_core(root_data: &'a T, depth: usize) -> Self {
        RoutingTable {
            map: HashMap::new(),
            data: root_data,
            depth,
        }
    }

    pub fn register(self: &mut Self, entity: &'a T, route: &'a [&str]) -> () {
        if route.len() > 0 {
            let wrapped_route_vec = route[1..].iter().map(|x| OneOrMore::One(x));
            let route_arr = Box::from_iter(wrapped_route_vec);
            self.register_one_core(entity, route[0], &route_arr);  // FIXME: Why is this &route_arr not dangling????
            // box is dropped here
        }
        else {
            panic!("Double registration error (empty route registration)")
        }
    }

    pub fn reg_more(self: &mut Self, entity: &'a T, route: &[OneOrMore<'a>]) -> () {
        if route.len() > 0 {
            self.register_more_core(entity, route[0], route[1..].into());
        }
        else {
            panic!("Double registration error (empty route registration)")
        }
    }

    pub fn reg_parallel(self: &mut Self, entity: &'a T, route: &[SerialOrParallel<'a>]) -> () {
        let mut one_or_more_arr = Vec::<OneOrMore>::new();
        for item in route {
            match item {
                SerialOrParallel::Parallel(str_arr) => {
                    one_or_more_arr.push(OneOrMore::More(str_arr))
                }
                SerialOrParallel::Serial(str_arr) => {
                    for &serial_piece in *str_arr {
                        one_or_more_arr.push(OneOrMore::One(serial_piece))
                    }
                }
            }
        }
        self.reg_more(entity, &one_or_more_arr)
    }

    fn register_one_core<'b>(self: &mut Self, entity: &'a T, next_rt: &'a str, rest_rt: &'b Box<[OneOrMore<'a>]>) -> () {

        if rest_rt.len() == 0 {
            let find_rt = self.map.get(next_rt);
            match find_rt {
                Some(_) => panic!("Double registration error"),
                None => {
                    self.map.insert(next_rt, RoutingTable::new_core(entity, self.depth+1));
                },
            };
        }
        else {
            let find_rt = self.map.get_mut(next_rt);
            match find_rt {
                Some(found_rt) => {
                    found_rt.register_more_core(entity, rest_rt[0], rest_rt[1..].into());
                }
                None => {
                    let mut implicit_layer = RoutingTable::new_core(self.data, self.depth+1);
                    implicit_layer.register_more_core(entity, rest_rt[0], rest_rt[1..].into());
                    self.map.insert(next_rt, implicit_layer);
                },
            };
        }
    }

    fn register_more_core(self: &mut Self, entity: &'a T, next_rt: OneOrMore<'a>, rest_rt: Box<[OneOrMore<'a>]>) -> () {
        match next_rt {
            OneOrMore::One(one_rt) => { 
                self.register_one_core(entity, one_rt, &rest_rt)
            }
            OneOrMore::More(more_rt) => { 
                for each_rt in more_rt { self.register_one_core(entity, each_rt, &rest_rt) }    
            }
        }
    }
    
    pub fn lookup(self: &'a Self, keys: &'a [&str]) -> Option<RTLookupResult<'a, T>> {
        self.lookup_core(keys, 0)
    }
    
    fn lookup_core(self: &'a Self, keys: &'a [&str], start: usize) -> Option<RTLookupResult<'a, T>> {
        let key_start = keys.get(start);
        //println!("{:?}[{}] = {:?}", keys, start, key_start );
        if let Some(key) = key_start {
            let next_map = self.map.get(key);
            if let Some(map) = next_map {
                return map.lookup_core(keys, start+1);
            }
        }
        Some(RTLookupResult {
            val: self.data,
            depth: self.depth,
            keys_used: start,
            keep_going: self,
        })
    }
    
}

pub trait Boring {
    fn boooooring() -> ();
}

impl<T: Debug> Boring for RoutingTable<'_, T> {
    fn boooooring() {}
}

mod test {

    use super::{RoutingTable, one, more, par, ser};

    const BOTTOM_FALLBACK: &i32 = &14; 
    const APP_API_V4_SIGNUP: &i32 = &15;
    const APP_API_V4_SIGNIN: &i32 = &16; 
    const APP_API_V4_SIGNOUT: &i32 = &17; 

    #[test]
    fn simple_case() {
        let mut rt = RoutingTable::new(BOTTOM_FALLBACK);
        rt.register(APP_API_V4_SIGNUP , &["api", "v4", "sign-up" ]);
        rt.register(APP_API_V4_SIGNIN , &["api", "v4", "sign-in" ]);
        rt.register(APP_API_V4_SIGNOUT, &["api", "v4", "sign-out"]);
        
        assert_eq!(rt.lookup(&[                              ]).unwrap().val, BOTTOM_FALLBACK);
        assert_eq!(rt.lookup(&["api"                         ]).unwrap().val, BOTTOM_FALLBACK);
        assert_eq!(rt.lookup(&["api", "v4"                   ]).unwrap().val, BOTTOM_FALLBACK);
        assert_eq!(rt.lookup(&["api", "v4", "sign-up"        ]).unwrap().val, APP_API_V4_SIGNUP);
        assert_eq!(rt.lookup(&["api", "v4", "sign-in"        ]).unwrap().val, APP_API_V4_SIGNIN);
        assert_eq!(rt.lookup(&["api", "v4", "sign-in", "tail"]).unwrap().val, APP_API_V4_SIGNIN);
        assert_eq!(rt.lookup(&["api", "v4", "sign-out"       ]).unwrap().val, APP_API_V4_SIGNOUT);
        assert_eq!(rt.lookup(&["api", "v4", "DNE"            ]).unwrap().val, BOTTOM_FALLBACK);
    }

    #[test]
    fn batch_register() {
        let gpp = more(&["GET", "POST", "PUT"]);
        let lr = more(&["localhost", "remote.org"]);
        let mut rt_more = RoutingTable::new(BOTTOM_FALLBACK);
        rt_more.reg_more(APP_API_V4_SIGNUP, &[gpp, lr, one("api"), one("v4"), one("sign-up")]);
        rt_more.reg_more(APP_API_V4_SIGNIN, &[gpp, lr, one("api"), one("v4"), one("sign-in")]);
        rt_more.reg_more(APP_API_V4_SIGNOUT, &[gpp, lr, one("api"), one("v4"), one("sign-out")]);

        // println!("{:#?}", rt_more);

        assert_eq!(rt_more.lookup(&["GET" , "localhost" ,                               ]).unwrap().val, BOTTOM_FALLBACK);
        assert_eq!(rt_more.lookup(&["GET" , "localhost" , "api"                         ]).unwrap().val, BOTTOM_FALLBACK);
        assert_eq!(rt_more.lookup(&["GET" , "localhost" , "api", "v4"                   ]).unwrap().val, BOTTOM_FALLBACK);
        assert_eq!(rt_more.lookup(&["GET" , "localhost" , "api", "v4", "sign-up"        ]).unwrap().val, APP_API_V4_SIGNUP);
        assert_eq!(rt_more.lookup(&["POST", "localhost" , "api", "v4", "sign-up"        ]).unwrap().val, APP_API_V4_SIGNUP);
        assert_eq!(rt_more.lookup(&["PUT" , "localhost" , "api", "v4", "sign-up"        ]).unwrap().val, APP_API_V4_SIGNUP);
        assert_eq!(rt_more.lookup(&["GET" , "remote.org", "api", "v4", "sign-up"        ]).unwrap().val, APP_API_V4_SIGNUP);
        assert_eq!(rt_more.lookup(&["GET" , "localhost" , "api", "v4", "sign-in"        ]).unwrap().val, APP_API_V4_SIGNIN);
        assert_eq!(rt_more.lookup(&["GET" , "localhost" , "api", "v4", "sign-in", "tail"]).unwrap().val, APP_API_V4_SIGNIN);
        assert_eq!(rt_more.lookup(&["GET" , "localhost" , "api", "v4", "sign-out"       ]).unwrap().val, APP_API_V4_SIGNOUT);
        assert_eq!(rt_more.lookup(&["GET" , "localhost" , "api", "v4", "DNE"            ]).unwrap().val, BOTTOM_FALLBACK);
    }

    #[test]
    fn batch_register_egonomic() {
        let gpp = par(&["GET", "POST", "PUT"]);
        let lr = par(&["localhost", "remote.org"]);
        let mut rt_more = RoutingTable::new(BOTTOM_FALLBACK);
        rt_more.reg_parallel(APP_API_V4_SIGNUP,  &[ gpp , lr , ser(&["api", "v4", "sign-up" ]) ] );
        rt_more.reg_parallel(APP_API_V4_SIGNIN,  &[ gpp , lr , ser(&["api", "v4", "sign-in" ]) ] );
        rt_more.reg_parallel(APP_API_V4_SIGNOUT, &[ gpp , lr , ser(&["api", "v4", "sign-out"]) ] );

        // println!("{:#?}", rt_more);

        assert_eq!(rt_more.lookup(&["GET" , "localhost" ,                               ]).unwrap().val, BOTTOM_FALLBACK);
        assert_eq!(rt_more.lookup(&["GET" , "localhost" , "api"                         ]).unwrap().val, BOTTOM_FALLBACK);
        assert_eq!(rt_more.lookup(&["GET" , "localhost" , "api", "v4"                   ]).unwrap().val, BOTTOM_FALLBACK);
        assert_eq!(rt_more.lookup(&["GET" , "localhost" , "api", "v4", "sign-up"        ]).unwrap().val, APP_API_V4_SIGNUP);
        assert_eq!(rt_more.lookup(&["POST", "localhost" , "api", "v4", "sign-up"        ]).unwrap().val, APP_API_V4_SIGNUP);
        assert_eq!(rt_more.lookup(&["PUT" , "localhost" , "api", "v4", "sign-up"        ]).unwrap().val, APP_API_V4_SIGNUP);
        assert_eq!(rt_more.lookup(&["GET" , "remote.org", "api", "v4", "sign-up"        ]).unwrap().val, APP_API_V4_SIGNUP);
        assert_eq!(rt_more.lookup(&["GET" , "localhost" , "api", "v4", "sign-in"        ]).unwrap().val, APP_API_V4_SIGNIN);
        assert_eq!(rt_more.lookup(&["GET" , "localhost" , "api", "v4", "sign-in", "tail"]).unwrap().val, APP_API_V4_SIGNIN);
        assert_eq!(rt_more.lookup(&["GET" , "localhost" , "api", "v4", "sign-out"       ]).unwrap().val, APP_API_V4_SIGNOUT);
        assert_eq!(rt_more.lookup(&["GET" , "localhost" , "api", "v4", "DNE"            ]).unwrap().val, BOTTOM_FALLBACK);
    }

    #[test]
    #[should_panic]
    fn double_registration_panic() {
        let gpp = more(&["GET", "POST", "PUT"]);
        let lr = more(&["localhost", "remote.org"]);
        let mut rt_panic = RoutingTable::new(BOTTOM_FALLBACK);
        rt_panic.reg_more(APP_API_V4_SIGNUP, &[gpp, lr, one("api"), one("v4"), one("sign-up")]);
        rt_panic.reg_more(APP_API_V4_SIGNUP, &[gpp, lr, one("api"), one("v4"), one("sign-up")]);
    }

    #[test]
    #[should_panic]
    fn empty_registration_panic() {
        let mut rt_panic = RoutingTable::new(BOTTOM_FALLBACK);
        rt_panic.reg_more(APP_API_V4_SIGNUP, &[]);
    }
}

