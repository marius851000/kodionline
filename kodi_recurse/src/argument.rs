use shell_escape::escape;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Default)]
pub struct AppArgument {
    pub command_name: String,
    pub args_order: Vec<&'static str>,
    pub short_version: HashMap<&'static str, &'static str>,
    pub args: HashMap<String, String>, //long option, value (string)
    pub bool_set: HashSet<String>,
    pub sub_command: Option<Box<AppArgument>>,
}

impl AppArgument {
    pub fn get_command_safe(&self) -> String {
        let mut result = Vec::new();
        self.add_argument_to_vec(&mut result);

        result
            .iter()
            .map(|x| escape(Cow::from(x)).to_string())
            .collect::<Vec<String>>()
            .join(" ")
    }

    fn add_argument_to_vec(&self, vec: &mut Vec<String>) {
        vec.push(self.command_name.clone());

        let mut inserted_keys = HashSet::new();
        for key in &self.args_order {
            if self.is_present(*key) {
                self.push_single_arg(*key, vec);
                inserted_keys.insert(key.to_string());
            }
        }

        for key in self.args.keys() {
            if !inserted_keys.contains(key) {
                self.push_single_arg(key, vec)
            }
        }

        for key in self.bool_set.iter() {
            if !inserted_keys.contains(key) {
                self.push_single_arg(key, vec)
            }
        }

        if let Some(sub_command) = &self.sub_command {
            sub_command.add_argument_to_vec(vec);
        }
    }

    fn push_single_arg(&self, key: &str, vec: &mut Vec<String>) {
        if let Some(short_key) = self.short_version.get(key) {
            vec.push(format!("-{}", short_key));
        } else {
            vec.push(format!("--{}", key));
        };

        if let Some(value) = self.args.get(key) {
            vec.push(value.to_string());
        } else if !self.bool_set.contains(key) {
            panic!("attempted to access a non existing key");
        };
    }

    pub fn value_of(&self, key: &str) -> Option<&str> {
        self.args.get(key).map(|x| x.as_str())
    }

    pub fn is_present(&self, key: &str) -> bool {
        self.args.contains_key(key) | self.bool_set.contains(key)
    }
}
#[test]
fn test_app_argument() {
    let app = AppArgument {
        command_name: "hello".into(),
        args_order: vec!["text", "bool", "another"],
        short_version: {
            let mut s = HashMap::new();
            s.insert("text", "t");
            s
        },
        bool_set: {
            let mut s = HashSet::new();
            s.insert("bool".into());
            s
        },
        args: {
            let mut a = HashMap::new();
            a.insert("text".to_string(), "hello, world".to_string());
            a.insert("another".to_string(), "tes\"t".to_string());
            a
        },
        sub_command: Some(Box::new(AppArgument {
            command_name: "sub_command".into(),
            args_order: Vec::new(),
            short_version: HashMap::new(),
            bool_set: HashSet::new(),
            args: {
                let mut a = HashMap::new();
                a.insert("sub".to_string(), "arg".to_string());
                a
            },
            sub_command: None,
        })),
    };

    assert_eq!(
        app.get_command_safe(),
        "hello -t \'hello, world\' --bool --another \'tes\"t\' sub_command --sub arg"
    ); //may be subject to change
}
