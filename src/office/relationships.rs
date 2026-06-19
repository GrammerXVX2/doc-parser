use quick_xml::Reader;
use quick_xml::events::Event;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relationship {
    pub id: String,
    pub rel_type: String,
    pub target: String,
    pub target_mode: Option<String>,
}

pub fn parse_relationships(xml: &str) -> anyhow::Result<Vec<Relationship>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut out = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) | Event::Empty(e) => {
                let name_buf = e.name().as_ref().to_vec();
                let name = name_buf.as_slice();
                let local = match name.iter().position(|b| *b == b':') {
                    Some(idx) => &name[idx + 1..],
                    None => name,
                };
                if local == b"Relationship" {
                    let mut id = String::new();
                    let mut rel_type = String::new();
                    let mut target = String::new();
                    let mut target_mode = None;

                    for attr in e.attributes().flatten() {
                        let key = attr.key.as_ref();
                        let value = String::from_utf8_lossy(attr.value.as_ref()).to_string();
                        let local_key = match key.iter().position(|b| *b == b':') {
                            Some(idx) => &key[idx + 1..],
                            None => key,
                        };
                        match local_key {
                            b"Id" => id = value,
                            b"Type" => rel_type = value,
                            b"Target" => target = value,
                            b"TargetMode" => target_mode = Some(value),
                            _ => {}
                        }
                    }

                    if !id.is_empty() {
                        out.push(Relationship {
                            id,
                            rel_type,
                            target,
                            target_mode,
                        });
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(out)
}
