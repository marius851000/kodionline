// ignore color, as it may cause contrast issue
static IGNORE_COLOR: bool = true;

#[must_use]
pub fn format_to_string(source: &str) -> String {
    let mut escaped = String::new();
    html_escape::encode_text_to_string(source, &mut escaped);

    let mut rendered_string = String::new();

    let mut chars = escaped.chars();
    while let Some(char) = chars.next() {
        if char == '[' {
            let mut first_keyword = String::new();
            let mut have_following_content = false;
            while let Some(char) = chars.next() {
                if char == ']' {
                    break;
                } else if char == ' ' {
                    have_following_content = true;
                    break;
                } else {
                    first_keyword.push(char)
                }
            }

            let mut extension = String::new();
            if have_following_content {
                //while let Some(char) = chars.next() {
                for char in &mut chars {
                    if char == ']' {
                        break;
                    } else {
                        extension.push(char);
                    }
                }
            }

            if first_keyword == "B" {
                rendered_string.push_str("<b>")
            } else if first_keyword == "/B" {
                rendered_string.push_str("</b>")
            } else if first_keyword == "COLOR" {
                if !IGNORE_COLOR {
                    let alpha: String = extension.drain(..2).collect();
                    let rgb: String = extension.drain(..6).collect();
                    rendered_string.push_str("<span style=\"color: #");
                    rendered_string.push_str(&rgb);
                    rendered_string.push_str(&alpha);
                    rendered_string.push_str(";\">");
                }
            } else if first_keyword == "/COLOR" {
                if !IGNORE_COLOR {
                    rendered_string.push_str("</span>");
                }
            } else {
                rendered_string.push('[');
                rendered_string.push_str(&first_keyword);
                if !extension.is_empty() {
                    rendered_string.push(' ');
                    rendered_string.push_str(&extension);
                };
                rendered_string.push(']');
            }
        } else {
            rendered_string.push(char)
        }
    }
    rendered_string
}

#[test]
fn test_format() {
    assert_eq!(&format_to_string("[B]Hello[/B]"), "<b>Hello</b>");
    assert_eq!(
        &format_to_string("[COLOR ffFBBA16]Hello[/COLOR]"),
        if IGNORE_COLOR {
            "Hello"
        } else {
            "<span style=\"color: #FBBA16ff;\">Hello</span>"
        }
    );
}
