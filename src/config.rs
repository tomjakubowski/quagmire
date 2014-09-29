use toml;

use std::collections::HashMap;

#[deriving(Decodable, Encodable)]
pub struct Config {
    macros: HashMap<String, Macro>
}

impl Config {
    pub fn new() -> Config {
        Config {
            macros: HashMap::new()
        }
    }

    pub fn new_from_path(path: Path) -> Config {
        use std::io::File;
        let contents = File::open(&path).read_to_string().unwrap();
        toml::decode_str(contents.as_slice()).unwrap()
    }

    pub fn expand_macro(&self, name: &str) -> Option<String> {
        self.macros.find_equiv(&name).map(|a| a.expand())
    }
}

/// A "simple" macro which expands directly to one or more lines to send to the MUD.
#[deriving(Decodable, Encodable)]
struct Macro {
    commands: Vec<String>
}

impl Macro {
    fn expand(&self) -> String {
        let mut cmds = self.commands.connect("\n");
        cmds.push('\n');
        cmds
    }
}

#[cfg(test)]
mod test {
    use super::Config;

    fn read_config(t: &str) -> Config {
        use toml::decode_str;
        decode_str(t).unwrap()
    }

    #[test]
    fn macros_simple() {
        let config = read_config(r#"
        [macros.foo]
        commands = ["look", "who"]
        "#);

        assert_eq!(config.expand_macro("foo"), Some("look\nwho".to_string()));
        assert_eq!(config.expand_macro("bar"), None);
    }
}
