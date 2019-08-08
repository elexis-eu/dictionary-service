use rand;
use rand::Rng;
use std::cmp::{Ord, Ordering};

/////////////////////////////////////////////////////////////////////////////////////////////
// Values

/// Any RDF value
#[derive(PartialEq,Debug,Clone,Eq,PartialOrd,Ord)]
pub enum Value {
    Literal(Literal),
    Resource(Resource)
}

impl Value {
    pub fn make_uri(s : &str) -> Value {
        Value::Resource(Resource::Named(NamedNode::URIRef(s.to_string())))
    }
    pub fn make_blank(s : &str) -> Value {
        Value::Resource(Resource::BlankNode(s.to_string()))
    }
    pub fn make_literal(s : &str) -> Value {
        Value::Literal(Literal::PlainLiteral(s.to_string()))
    }
    pub fn make_lang_literal(s : &str, l : & str) -> Value {
        Value::Literal(Literal::LangLiteral(s.to_string(), l.to_string()))
    }
    pub fn make_typed_literal(s : &str, t : NamedNode) -> Value {
        Value::Literal(Literal::TypedLiteral(s.to_string(), t))
    }

}

impl ToString for Value {
    fn to_string(&self) -> String {
        match self {
            Value::Literal(l) => l.to_string(),
            Value::Resource(r) => r.to_string()
        }
    }
}

impl From<Resource> for Value {
    fn from(r : Resource) -> Self {
        Value::Resource(r)
    }
}

impl From<Literal> for Value {
    fn from(l : Literal) -> Self {
        Value::Literal(l)
    }
}

impl From<NamedNode> for Value {
    fn from(n : NamedNode) -> Self {
        Value::Resource(Resource::Named(n))
    }
}

/// A RDF resource
#[derive(PartialEq,Debug,Clone,Eq,Ord,PartialOrd)]
pub enum Resource {
    BlankNode(String),
    Named(NamedNode)
}

impl Resource {
    pub fn add_property(&self, pred : NamedNode, value : Value) -> Triple {
        Triple(self.clone(), pred, value)
    }

    pub fn make_uri(s : &str) -> Resource {
        Resource::Named(NamedNode::URIRef(s.to_string()))
    }
    pub fn make_blank(s : &str) -> Resource {
        Resource::BlankNode(s.to_string())
    }
    pub fn make_anon() -> Resource {
        let mut r = rand::thread_rng();
        let x : u64 = r.gen();
        Resource::BlankNode(format!("{:016x}", x))
    }
    pub fn as_value(self) -> Value {
        Value::Resource(self)
    }
    pub fn is_uri(&self) -> bool {
        match self {
            Resource::BlankNode(_) => false,
            Resource::Named(_) => true
        }
    }
    pub fn is_bnode(&self) -> bool {
        match self {
            Resource::BlankNode(_) => true,
            Resource::Named(_) => false
        }
    }
}

impl ToString for Resource {
    fn to_string(&self) -> String {
        match self {
            Resource::BlankNode(id) => format!("_:{}", id),
            Resource::Named(n) => n.to_string()
        }
    }
}

/// A Resource (non-literal) RDF value
#[derive(Debug,Clone,Eq)]
pub enum NamedNode {
    URIRef(String),
    QName(Namespace, String)
}

impl NamedNode {
    pub fn uri(&self) -> String {
        match self {
            NamedNode::URIRef(uri) => uri.to_string(),
            NamedNode::QName(Namespace(_, prefix), suffix) => format!("{}{}", prefix, suffix)
        }
    }
    pub fn make_uri(s : &str) -> NamedNode {
        NamedNode::URIRef(s.to_string())
    }
    pub fn as_resource(self) -> Resource {
        Resource::Named(self)
    }
    pub fn as_value(self) -> Value {
        Value::Resource(Resource::Named(self))
    }
}

impl Ord for NamedNode {
    fn cmp(&self, other : &Self) -> Ordering {
        self.uri().cmp(&other.uri())
    }
}

impl PartialOrd for NamedNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for NamedNode {
    fn eq(&self, other : &Self) -> bool {
        self.uri() == other.uri()
    }
}

impl ToString for NamedNode {
    fn to_string(&self) -> String {
        match self {
            NamedNode::URIRef(uri) => format!("<{}>", uri),
            NamedNode::QName(Namespace(abbrev, _), suffix) => format!("{}:{}", abbrev, suffix)
        }
    }
}

/// A literal RDF value
#[derive(PartialEq,Debug,Clone,Eq,PartialOrd,Ord)]
pub enum Literal {
    PlainLiteral(String),
    LangLiteral(String, String),
    TypedLiteral(String, NamedNode)
}

impl Literal {
    pub fn string_value<'a>(&'a self) -> &'a str {
        match self {
            Literal::PlainLiteral(lit) => lit,
            Literal::LangLiteral(lit,_) => lit,
            Literal::TypedLiteral(lit,_) => lit
        }
    }
    pub fn as_value(self) -> Value {
        Value::Literal(self)
    }
}

impl ToString for Literal {
    fn to_string(&self) -> String {
        match self {
            Literal::PlainLiteral(lit) => format!("\"{}\"", lit),
            Literal::LangLiteral(lit,lang) => format!("\"{}\"@{}", lit, lang),
            Literal::TypedLiteral(lit,datatype) => format!("\"{}\"^^{}", lit, datatype.to_string())
        }
    }
}

#[derive(PartialEq,Debug,Clone,Eq)]
pub struct Namespace(pub String,pub String);

impl Namespace {
    pub fn new(abbrev : &str, prefix : &str) -> Namespace {
        Namespace(abbrev.to_string(), prefix.to_string())
    }
    pub fn make_resource(&self, suffix : &str) -> Resource {
        Resource::Named(NamedNode::QName(self.clone(), suffix.to_string()))
    }

    pub fn make_named_node(&self, suffix : &str) -> NamedNode {
        NamedNode::QName(self.clone(), suffix.to_string())
    }

    pub fn make_value(&self, suffix : &str) -> Value {
        Value::Resource(Resource::Named(NamedNode::QName(self.clone(), suffix.to_string())))
    }
}

#[derive(PartialEq,Debug,Clone,Eq,PartialOrd,Ord)]
pub struct Triple(pub Resource,pub NamedNode,pub Value);

