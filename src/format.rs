// ignore color, as it may cause contrast issue
static IGNORE_COLOR: bool = true;

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
                while let Some(char) = chars.next() {
                    if char == ']' {
                        break;
                    } else {
                        extension.push(char);
                    }
                }
            }

            if first_keyword == "B" {
                rendered_string.extend("<b>".chars())
            } else if first_keyword == "/B" {
                rendered_string.extend("</b>".chars())
            } else if first_keyword == "COLOR" {
                if !IGNORE_COLOR {
                    let alpha: String = extension.drain(..2).collect();
                    let rgb: String = extension.drain(..6).collect();
                    rendered_string.extend("<span style=\"color: #".chars());
                    rendered_string.extend(rgb.chars());
                    rendered_string.extend(alpha.chars());
                    rendered_string.extend(";\">".chars());
                }
            } else if first_keyword == "/COLOR" {
                if !IGNORE_COLOR {
                    rendered_string.extend("</span>".chars());
                }
            } else {
                rendered_string.extend("[".chars());
                rendered_string.extend(first_keyword.chars());
                if extension.len() > 0  {
                    rendered_string.push(' ');
                    rendered_string.extend(extension.chars());
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
        "<span style=\"color: #FBBA16ff;\">Hello</span>"
    );
    //TODO: test for [COLOR]
}
