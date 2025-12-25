pub mod ast;
pub mod parser;

pub use parser::parser;
pub use ast::*;

#[cfg(test)]
mod tests {
    use super::*;
    use chumsky::Parser;

    #[test]
    fn test_tcp_flags() {
        let src = r#"
            struct TcpFlags {
                data_offset: b<4>,
                reserved: b<3>,  
                nonce: b<1>,
                cwr: b<1>,
                ecn: b<1>,
                urg: b<1>,
                ack: b<1>,
                psh: b<1>,
                rst: b<1>,
                syn: b<1>,
                fin: b<1>,
                window_size: b<16>, 
            }
        "#;
        
        let result = parser().parse(src).into_result();
        assert!(result.is_ok(), "Parse errors: {:?}", result.err());
        let defs = result.unwrap();
        assert_eq!(defs.len(), 1);
        match &defs[0] {
            Definition::Struct(s) => {
                assert_eq!(s.name, "TcpFlags");
                assert_eq!(s.items.len(), 12);
            }
            _ => panic!("Expected struct"),
        }
    }

    #[test]
    fn test_tlv() {
        let src = r#"
            struct Tlv {
                tag: u8,
                len: u16,
                value: [u8; (len * 2) - 4], 
                @no_cache
                trailer: @greedy(unsafe_eof) [u8],
            }
        "#;
        
        let result = parser().parse(src).into_result();
        assert!(result.is_ok(), "Parse errors: {:?}", result.err());
    }

    #[test]
    fn test_union() {
        let src = r#"
            struct IcmpPacket {
                type: u8,
                code: u8,
                checksum: u16,
                
                body: union(type) {
                    0 | 8 => Echo { 
                        id: u16, 
                        seq: u16, 
                        payload: @greedy [u8]
                    },
                    3 => DestUnreach { 
                        unused: u32, 
                        orig_header: @greedy [u8]
                    },
                    _ => Raw { data: @greedy [u8] },
                }
            }
        "#;
         let result = parser().parse(src).into_result();
         assert!(result.is_ok(), "Parse errors: {:?}", result.err());
    }
    
    #[test]
    fn test_conditional() {
        let src = r#"
            struct ConstBitExample {
                reserved: b<3> = b000, 
                magic: u8 = xFF,
                version: u8 = 10,

                mode: b<3>,
                if (mode == b101) {
                    special_param: u8,
                }
            }
        "#;
        let result = parser().parse(src).into_result();
        assert!(result.is_ok(), "Parse errors: {:?}", result.err());
    }
}
