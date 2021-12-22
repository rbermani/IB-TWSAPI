//! Serde IB TWS Server data type deserialization

use crate::serde_tws::error::*;
use std::convert::TryInto;
use crate::core::messages::ServerReqMsg;

use std::iter::Peekable;
//use std::slice::Iter;
//use std::iter::IntoIterator;
//use::alloc_vec::IntoIter;

use serde::de::{
    self, value::U8Deserializer, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess,
    SeqAccess, VariantAccess, Visitor,
};
use serde::Deserialize;

pub fn from_bytes<'a, T>(b: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    Deserializer::from_bytes(b).deserialize()
}

#[derive(Clone)]
pub struct Deserializer<'de> {
    source: &'de [u8],
    payload_len: usize,
    veclen: usize,
    field_iter: Peekable<std::vec::IntoIter<&'de [u8]>>,
}

impl<'de> Deserializer<'de> {
    pub fn from_bytes(input: &'de [u8]) -> Self {
        let payload_len = i32::from_be_bytes(input[0..4].try_into().unwrap()) as usize;
        let fields: Vec<&[u8]> = input[4..].split(|val| val == &(0 as u8)).collect();
        let field_iter = fields.into_iter().peekable();
        let veclen = 0;
        Deserializer {
            source: input,
            payload_len,
            veclen,
            field_iter,
        }
    }

    pub fn deserialize<T>(mut self) -> Result<T, Error>
    where
        T: Deserialize<'de>,
    {
        T::deserialize(&mut self)
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    // IB TWS data types are not self describing
    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let next = self.field_iter.next();
        let nextval_str = std::str::from_utf8(next.unwrap()).unwrap();
        let val: bool = nextval_str.parse().unwrap_or(false) as bool;
        visitor.visit_bool(val)
    }

    fn deserialize_i8<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_i16<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let next = self.field_iter.next();
        let nextval_str = std::str::from_utf8(next.unwrap()).unwrap();
        visitor.visit_i32(nextval_str.parse().unwrap_or(0))
    }

    fn deserialize_i64<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_u8<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_u16<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_u32<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_u64<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_f64(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let next = self.field_iter.next();
        let nextval_str = std::str::from_utf8(next.unwrap()).unwrap();
        visitor.visit_f64(nextval_str.parse().unwrap_or(0.0))
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        println!("deserialize_str()");
        let next = self.field_iter.next();
        let nextval_str = std::str::from_utf8(next.unwrap()).unwrap();
        visitor.visit_borrowed_str(nextval_str)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let next = self.field_iter.next();
        let nextval_str = std::str::from_utf8(next.unwrap()).unwrap();
        let result: i32 = nextval_str.parse().unwrap_or(0);
        if result == 0 {
            visitor.visit_none()
        } else if result == 1 {
            visitor.visit_some(self)
        } else {
            // undefined case
            Err(Error::ExpectedBoolean)
        }
    }

    // In Serde, unit means an anonymous value containing no data.
    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::ExpectedNull)
    }

    // Unit struct means a named value containing no data.
    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    // As is done here, serializers are encouraged to treat newtype structs as
    // insignificant wrappers around the data they contain. That means not
    // parsing anything other than the contained value.
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    // Deserialization of compound types like sequences and maps happens by
    // passing the visitor an "Access" object that gives it the ability to
    // iterate through the data contained in the sequence.
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let Some(i) = self.field_iter.next() {
            let nextval_str = std::str::from_utf8(i).unwrap();
            self.veclen = usize::from_str_radix(nextval_str, 10)?;
            visitor.visit_seq(VecSeqAccess::new(self))
        } else {
            Err(Error::ExpectedArray)
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::ExpectedMapEnd)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        println!("deserialize_struct()");
        visitor.visit_seq(self)
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        println!(
            "deserialize_enum() {} inputlen: {} payloadlen: {}",
            name,
            self.source.len(),
            self.payload_len
        );
        if name.eq("ServerReqMsg") {
            let next = self.field_iter.next().unwrap();
            let nextval_str = std::str::from_utf8(next).unwrap();
            let mut msg_id_idx: usize = nextval_str.parse().unwrap_or(0);
            println!("PreMsg ID: {}", msg_id_idx);

            if msg_id_idx >= 49 && msg_id_idx < 60 {
                msg_id_idx -= 23;
            } else if msg_id_idx >= 61 && msg_id_idx < 100 {
                msg_id_idx -= 24;
            } else if msg_id_idx >= 100 {
                return Err(Error::Unsupported);
            }

            msg_id_idx -= 1;
            println!("Using Msg ID: {}", msg_id_idx);
            visitor.visit_enum(Enum::new(self, msg_id_idx as u8))
        } else {
            let next = self.field_iter.next().unwrap();
            let nextval_str = std::str::from_utf8(next).unwrap();
            let mut msg_id_idx: usize = nextval_str.parse().unwrap_or(0);
            println!("Enum ID: {}", msg_id_idx);
            visitor.visit_enum(Enum::new(self, msg_id_idx as u8))
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
}

struct VecSeqAccess<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    len: usize,
}

impl<'a, 'de> VecSeqAccess<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        let len: usize = de.veclen;
        VecSeqAccess { de, len }
    }
}

impl<'de, 'a> SeqAccess<'de> for VecSeqAccess<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        //print_type_of(&seed);
        if self.len > 0 {
            self.len -= 1;
            return seed.deserialize(&mut *self.de).map(Some);
        } else {
            return Ok(None);
        }
    }
}

impl<'de> MapAccess<'de> for &mut Deserializer<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        // Deserialize a map key.
        seed.deserialize(&mut **self).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut **self)
    }
}

impl<'de> SeqAccess<'de> for Deserializer<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        match self.field_iter.peek() {
            None => Ok(None),
            Some(&_s) => seed.deserialize(self).map(Some),
        }
    }
}

struct Enum<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    index: u8,
}

impl<'a, 'de> Enum<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, index: u8) -> Self {
        Enum { de, index }
    }
}

impl<'de, 'a> EnumAccess<'de> for Enum<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self)>
    where
        V: DeserializeSeed<'de>,
    {
        println!("variant_seed()");
        let tmpde: U8Deserializer<Self::Error> = self.index.into_deserializer();
        let v = seed.deserialize(tmpde)?;
        Ok((v, self))
    }
}

impl<'de, 'a> VariantAccess<'de> for Enum<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        println!("newtype_variant_seed()");
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        println!("newtype_variant_seed()");
        let value = seed.deserialize(self.de)?;
        Ok(value)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        println!("tuple_variant()");
        de::Deserializer::deserialize_tuple(self.de, len, visitor)
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        println!("struct_variant()");
        de::Deserializer::deserialize_struct(self.de, "", fields, visitor)
    }
}

/*
#[test]
fn test_struct() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Test {
        int: u32,
        seq: Vec<String>,
    }

    let j = r#"{"int":1,"seq":["a","b"]}"#;
    let expected = Test {
        int: 1,
        seq: vec!["a".to_owned(), "b".to_owned()],
    };
    assert_eq!(expected, from_str(j).unwrap());
}

#[test]
fn test_enum() {
    #[derive(Deserialize, PartialEq, Debug)]
    enum E {
        Unit,
        Newtype(u32),
        Tuple(u32, u32),
        Struct { a: u32 },
    }

    let j = r#""Unit""#;
    let expected = E::Unit;
    assert_eq!(expected, from_str(j).unwrap());

    let j = r#"{"Newtype":1}"#;
    let expected = E::Newtype(1);
    assert_eq!(expected, from_str(j).unwrap());

    let j = r#"{"Tuple":[1,2]}"#;
    let expected = E::Tuple(1, 2);
    assert_eq!(expected, from_str(j).unwrap());

    let j = r#"{"Struct":{"a":1}}"#;
    let expected = E::Struct { a: 1 };
    assert_eq!(expected, from_str(j).unwrap());
}
*/
