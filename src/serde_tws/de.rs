//! Serde IB TWS Server data type deserialization

use crate::serde_tws::error::*;
use crate::core::messages::{ServerRspMsg,ServerRspMsgDiscriminants};
use std::iter::Peekable;
use std::convert::TryInto;
//use std::slice::Iter;
//use std::iter::IntoIterator;
//use::alloc_vec::IntoIter;
use std::ops::{AddAssign, MulAssign, Neg};
use serde::de::{
    self, EnumAccess, IntoDeserializer, MapAccess, SeqAccess,
    VariantAccess, Visitor, DeserializeSeed
};
use serde::Deserialize;
use strum::VariantNames;

fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}

pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    Deserializer::from_str(s).deserialize()
    //let mut deserializer = Deserializer::from_str(s);
    //let t = T::deserialize(&mut deserializer)?;
    // if deserializer.input.is_empty() {
    //     Ok(t)
    // } else {
    //     Err(Error::TrailingCharacters)
    // }
    //Err(Error::TrailingCharacters)
}

pub struct Deserializer<'de> {
    source: &'de str,
    offset: usize,
    payload_len: usize,
    field_iter: Peekable<std::vec::IntoIter<&'de str>>
}

impl<'de> Deserializer<'de> {
    pub fn from_str(input: &'de str) -> Self {
        let bytes = input.as_bytes();
        let payload_len = i32::from_be_bytes(bytes[0..4].try_into().unwrap()) as usize;
        let fields: Vec<&str> = input[4..]
            .split('\0')
            .collect();
        let field_iter = fields.into_iter().peekable();
        //print_type_of(&field_iter);
        Deserializer {
            source: input,
            payload_len,
            offset: 0,
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

impl<'de> Deserializer<'de> {
    fn expect_end(&mut self) -> Result<()> {
        match self.field_iter.next() {
            None => Ok(()),
            Some(s) => Err(Error::TrailingBytes),
        }
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

    fn deserialize_bool<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
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
        visitor.visit_i32(next.unwrap().parse().unwrap_or(0))
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
        visitor.visit_f64(next.unwrap().parse().unwrap_or(0.0))
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
        visitor.visit_borrowed_str(next.unwrap())
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
        visitor.visit_some(self)
    }

    // In Serde, unit means an anonymous value containing no data.
    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::ExpectedNull)
    }

    // Unit struct means a named value containing no data.
    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    // As is done here, serializers are encouraged to treat newtype structs as
    // insignificant wrappers around the data they contain. That means not
    // parsing anything other than the contained value.
    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value>
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
        visitor.visit_seq(self)

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
        //self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {

        println!("deserialize_enum() inputlen: {} payloadlen: {}", self.source.len(), self.payload_len);
        visitor.visit_enum(self)

    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let next = self.field_iter.next();
        let msg_id_idx: usize = next.unwrap().parse().unwrap_or(0) - 1;
        //println!("deserialize_identifier() msg: {} len {}", msg_id_idx, ServerRspMsgDiscriminants::VARIANTS.len());

        if msg_id_idx < ServerRspMsgDiscriminants::VARIANTS.len() {
            let variant_name = ServerRspMsgDiscriminants::VARIANTS[msg_id_idx];
            visitor.visit_borrowed_str(variant_name)
        } else {
            Err(Error::ExpectedMapEnd)
        }
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
}

impl<'de> EnumAccess<'de> for &mut Deserializer<'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self)>
    where
        V: DeserializeSeed<'de>,
    {
        println!("variant_seed()");
        Ok((seed.deserialize(&mut *self)?, self))
    }
}

impl<'de> SeqAccess<'de> for Deserializer<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {

        match self.field_iter.peek() {
            None => {
                 println!("next_element_seed() NO MORE elements left");
                return Ok(None);
            },
            Some(&_s) => {
                println!("RB next_element_seed");
                return seed.deserialize(self).map(Some);
            },
        };

        //self.len -= 1;
        //Ok(None)
        //seed.deserialize(self).map(Some)
    }
}

impl<'de> VariantAccess<'de> for &mut Deserializer<'de> {
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
        let value = seed.deserialize(&mut *self)?;
        self.expect_end()?;
        Ok(value)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        println!("tuple_variant()");
        let value = serde::de::Deserializer::deserialize_seq(&mut *self, visitor)?;
        self.expect_end()?;
        Ok(value)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        println!("struct_variant()");
        let value = serde::de::Deserializer::deserialize_seq(&mut *self, visitor)?;

        //let value = serde::de::Deserializer::deserialize_map(&mut *self, visitor)?;
        //self.expect_end()?;
        Ok(value)
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
