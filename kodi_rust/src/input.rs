use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};

pub fn decode_input(inputs_option: &str) -> Vec<String> {
    if inputs_option.len() == 0 {
        Vec::new()
    } else {
        let mut result = Vec::new();
        for input in inputs_option.split(':') {
            result.push(percent_decode_str(input).decode_utf8_lossy().into());
            //TODO: maybe catch the error someway, just decide what to do in this case
        }
        result
    }
}

pub fn encode_input(inputs: &[String]) -> String {
    let mut result = String::new();
    for (input_nb, input) in inputs.iter().enumerate() {
        if input_nb != 0 {
            result.push(':');
        };
        result.push_str(&utf8_percent_encode(input, NON_ALPHANUMERIC).to_string());
    }
    result
}
