use pest::Parser;
use pest::iterators::Pair;
use pest;
use crate::rdf::model::*;
use std::collections::HashMap;
use std::result;

type Result<T> = result::Result<T, TurtleParserError>;

#[derive(Parser)]
#[grammar = "rdf/turtle.pest"]
struct TurtleParser;

pub fn parse_turtle(data : &str) -> Result<Vec<Triple>> {
    let pairs = TurtleParser::parse(Rule::turtle_doc, data)
        .map_err(|e| TurtleParserError::Parse(format!("{}", e)))?;

    let mut state = ParserState {
        triples : Vec::new(),
        namespaces : HashMap::new(),
        bnodes : 0
    };

    let mut read = 0;


    for pair in pairs {
        read = pair.clone().into_span().end();
        for pair2 in pair.into_inner() {
            if pair2.as_rule() == Rule::directive {
                process_directive(pair2, &mut state);
            } else if pair2.as_rule() == Rule::triples {
                process_triples(pair2, &mut state)?;
            }
        }

    }
    if read < data.len() {
        Err(TurtleParserError::ParseIncomplete(read))
    } else {
        Ok(state.triples)
    }
}

#[derive(Debug)]
struct ParserState {
    triples : Vec<Triple>,
    namespaces : HashMap<String, Namespace>,
    bnodes : usize
}

fn process_directive<'i>(p : Pair<'i, Rule>, state : &mut ParserState) {
    for p2 in p.into_inner() {
        if p2.as_rule() == Rule::prefix_id {
            let mut p3 = p2.into_inner();
            let prefix = p3.next().expect("grammar error (prefix)");
            let uriref = p3.next().expect("grammar error (prefix uriref)");
            let full = process_uriref(uriref);
            let ns = Namespace(prefix.as_str().to_string(), full[1..(full.len()-1)].to_string());
            state.namespaces.insert(prefix.as_str().to_string(), ns);
        } else if p2.as_rule() == Rule::base {
            let mut p3 = p2.into_inner();
            let uriref = p3.next().expect("grammar error (base uriref)");
            let ns = Namespace("@base".to_string(), process_uriref(uriref));
            state.namespaces.insert("@base".to_string(), ns);
        } else {
            panic!("grammar error (directive)");
        }
    }

}

fn process_uriref<'i>(p : Pair<'i, Rule>) -> String {
    //p.into_inner().next().expect("grammar error (uriref)").as_str().to_string()
    p.as_str().to_string()
}

fn process_triples<'i>(p : Pair<'i, Rule>, state : &mut ParserState) -> Result<()> {
    let mut p3 = p.into_inner();
    let subject = {
        let p4 = p3.next().expect("grammar error (subject)");
        if p4.as_rule() == Rule::resource {
            process_resource(p4, state)?.as_resource()
        } else if p4.as_rule() == Rule::blank {
            process_blank(p4.into_inner().next().expect("grammar error (subject/blank"), state)?
        } else {
            panic!("grammar error (subject2)")
        }
    };
    process_predicate_object_list(p3.next().expect("grammar error (predicate_object_list)"),
        state, subject)
}

fn process_resource<'i>(p : Pair<'i, Rule>, state : &mut ParserState) -> Result<NamedNode> {
    process_named_node(p.into_inner().next().unwrap(), state)
}

fn process_named_node<'i>(p : Pair<'i, Rule>, state : &mut ParserState) -> Result<NamedNode> {
    if p.as_rule() == Rule::uriref {
        let s = p.as_str();
        Ok(NamedNode::make_uri(&s[1..(s.len()-1)]))
    } else if p.as_rule() == Rule::qname {
        let mut p2 = p.into_inner();
        let s1 = p2.next().expect("grammar error (qname prefix)").as_str();
        let (pre, suf) = match p2.next() {
            Some(suffix) => (s1.to_string(), suffix.as_str().to_string()),
            None => ("".to_string(), s1.to_string())
        };
        match state.namespaces.get(&pre) {
            Some(ns) => Ok(ns.make_named_node(&suf)),
            None => Err(TurtleParserError::NamespaceNotFound(pre.to_string()))
        }

    } else {
        eprintln!("{:?}", p.as_str());
        eprintln!("{:?}", p.as_rule());
        panic!("grammar error (resource)");
    }
}

fn process_blank<'i>(p : Pair<'i, Rule>, state : &mut ParserState) -> Result<Resource> {
    if p.as_rule() == Rule::blank_node_id {
        let s = p.as_str();
        Ok(Resource::make_blank(&s[2..]))
    } else if p.as_rule() == Rule::blank_node_empty {
        let id = format!("nodeID{}", state.bnodes);
        state.bnodes += 1;
        Ok(Resource::make_blank(&id))
    } else if p.as_rule() == Rule::blank_node_preds {
        let id = format!("nodeID{}", state.bnodes);
        state.bnodes += 1;
        let subj = Resource::make_blank(&id);
        process_predicate_object_list(p.into_inner().next().unwrap(), state, subj.clone())?;
        Ok(subj)
    } else if p.as_rule() == Rule::blank_node_collection {
        let id = format!("nodeID{}", state.bnodes);
        state.bnodes += 1;
        let mut node = Resource::make_blank(&id);

        let objects = process_object_list(p.into_inner().next().unwrap(), state)?;
        let mut first = true;
        for obj in objects {
            if !first {
                let id2 = format!("nodeID{}", state.bnodes);
                state.bnodes += 1;
                let node2 = Resource::make_blank(&id2);
                state.triples.push(Triple(node.clone(),
                                          NamedNode::make_uri("http://www.w3.org/1999/02/22-rdf-syntax-ns#rest"), node2.clone().as_value()));
                node = node2;
                state.triples.push(Triple(
                    node.clone(), NamedNode::make_uri("http://www.w3.org/1999/02/22-rdf-syntax-ns#first"), obj));
            } else {
                state.triples.push(Triple(
                    node.clone(), NamedNode::make_uri("http://www.w3.org/1999/02/22-rdf-syntax-ns#first"), obj));
                first = false;
            }
        }
        if first {
            Ok(Resource::make_uri("http://www.w3.org/1999/02/22-rdf-syntax-ns#nil"))
        } else {
            state.triples.push(Triple(node,
                                      NamedNode::make_uri("http://www.w3.org/1999/02/22-rdf-syntax-ns#rest"),
                                      Resource::make_uri("http://www.w3.org/1999/02/22-rdf-syntax-ns#nil").as_value()));
            Ok(Resource::make_blank(&id))
        }
    } else if p.as_rule() == Rule::blank_node_collection_empty {
        Ok(Resource::make_uri("http://www.w3.org/1999/02/22-rdf-syntax-ns#nil"))
    } else {
        panic!("")
    }
}

fn process_predicate_object_list<'i>(p : Pair<'i, Rule>, state : &mut ParserState,
                subj : Resource) -> Result<()> {
    let mut p2 = p.into_inner();
    loop {
        let verb = match p2.next() {
            Some(v) => process_verb(v, state)?,
            None => break
        };
        let objects = process_object_list(p2.next().expect("Grammar error (objects)"), state)?;
        for obj in objects {
            state.triples.push(Triple(subj.clone(), verb.clone(), obj))
        }
    }
    Ok(())
}

fn process_verb<'i>(p : Pair<'i, Rule>, state : &mut ParserState) -> Result<NamedNode> {
    let mut p2 = p.into_inner();
    match p2.next() {
        Some(res) => process_named_node(res.into_inner().next().expect("grammar error (verb/resource)"), state),
        None => Ok(NamedNode::make_uri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"))
    }
}

fn process_object_list<'i>(p : Pair<'i, Rule>, state : &mut ParserState) -> Result<Vec<Value>> {
    let mut objects = Vec::new();
    for p2 in p.into_inner() {
       objects.push(process_object(p2, state)?);
    }
    Ok(objects)
}

fn process_object<'i>(p : Pair<'i, Rule>, state : &mut ParserState) -> Result<Value> {
   if p.as_rule() == Rule::resource {
       Ok(process_named_node(p.into_inner().next().expect("grammar error (object/resource)"), state)?.as_value())
   } else if p.as_rule() == Rule::blank {
       Ok(process_blank(p.into_inner().next().expect("grammar error (object/blank)"), state)?.as_value())
   } else if p.as_rule() == Rule::lit {
       Ok(process_lit(p.into_inner().next().expect("grammar error (object/lit)"), state)?.as_value())
   } else {
       panic!("grammar error (object)")
   }
}

fn process_lit<'i>(p : Pair<'i, Rule>, state : &mut ParserState) -> Result<Literal> {
    if p.as_rule() == Rule::datatype_string {
        let mut p2 = p.into_inner();
        let str = process_quoted_string(p2.next().expect("grammar error (lit/qstring1)"));
        let dtype = process_resource(p2.next().expect("grammar error (lit/dtype)"), state)?;
        Ok(Literal::TypedLiteral(str, dtype))
    } else if p.as_rule() == Rule::lang_string {
        let mut p2 = p.into_inner();
        let str = process_quoted_string(p2.next().unwrap());
        let lang = p2.next().map(|x| {
            let s = x.as_str();
            s[1..].to_string()
        });
        match lang {
            Some(l) => Ok(Literal::LangLiteral(str, l)),
            None => Ok(Literal::PlainLiteral(str))
        }
    } else if p.as_rule() == Rule::integer {
        Ok(Literal::TypedLiteral(p.as_str().to_string(),
                                 NamedNode::make_uri("http://www.w3.org/2001/XMLSchema#integer")))
    } else if p.as_rule() == Rule::duble {
        Ok(Literal::TypedLiteral(p.as_str().to_string(),
                                 NamedNode::make_uri("http://www.w3.org/2001/XMLSchema#double")))
    } else if p.as_rule() == Rule::bool {
        Ok(Literal::TypedLiteral(p.as_str().to_string(),
                                 NamedNode::make_uri("http://www.w3.org/2001/XMLSchema#boolean")))
    } else {
        eprintln!("{:?}", p.as_rule());
        panic!("grammar error (lit)")
    }
}

fn process_quoted_string<'i>(p : Pair<'i, Rule>) -> String {
    if p.as_rule() == Rule::string {
        let s = p.as_str();
        s[1..(s.len() - 1)].to_string()
    } else if p.as_rule() == Rule::long_string {
        let s = p.as_str();
        s[3..(s.len() - 3)].to_string()
    } else {
        eprintln!("{:?}", p.as_rule());
        panic!("grammar error (quoted_string)")
    }
}


quick_error! {
    #[derive(Debug)]
    pub enum TurtleParserError {
        Parse(msg : String) {
            description("Parsing error")
            display("Could not parse: {}", msg)
        }
        NamespaceNotFound(namespace : String) {
            description("Namespace was not declared")
            display("Namespace not found ({})", namespace)
        }
        ParseIncomplete(read : usize) {
            description("Parse did not complete")
            display("Parse failed at {}", read)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_turtle() {
        let result = parse_turtle("@prefix foo: <bar> . foo:bar <baz> <x> .");
        assert_eq!(true, result.is_ok());
        assert!(result.unwrap().len() > 0);
    }

    #[test]
    fn test_literals() {
        let result = parse_turtle("<foo> <bar> \"plain\" , \"lang\"@en, \"typed\"^^<foo> , 0, 0.2e-6, false .");
        assert_eq!(result.unwrap(), vec![
            Triple(Resource::make_uri("foo"), NamedNode::make_uri("bar"), Value::make_literal("plain")),
            Triple(Resource::make_uri("foo"), NamedNode::make_uri("bar"), Value::make_lang_literal("lang", "en")),
            Triple(Resource::make_uri("foo"), NamedNode::make_uri("bar"), Value::make_typed_literal("typed", NamedNode::make_uri("foo"))),
            Triple(Resource::make_uri("foo"), NamedNode::make_uri("bar"), Value::make_typed_literal("0", NamedNode::make_uri("http://www.w3.org/2001/XMLSchema#integer"))),
            Triple(Resource::make_uri("foo"), NamedNode::make_uri("bar"), Value::make_typed_literal("0.2e-6", NamedNode::make_uri("http://www.w3.org/2001/XMLSchema#double"))),
            Triple(Resource::make_uri("foo"), NamedNode::make_uri("bar"), Value::make_typed_literal("false", NamedNode::make_uri("http://www.w3.org/2001/XMLSchema#boolean")))
            ]);

    }

    #[test]
    fn test_prefix() {
        let result = parse_turtle("@prefix foo: <bar> . foo:bar <bar> <x> .");
        let ns = Namespace::new("foo", "bar");
        assert_eq!(result.unwrap(), vec![Triple(ns.make_resource("bar"), NamedNode::make_uri("bar"), Value::make_uri("x"))]);
    }

    #[test]
    fn test_base() {
        // TODO
    }

    #[test]
    fn test_continuations() {
        let result = parse_turtle("<foo> <bar> <a> ; <b> <c> , <d> .");
        assert_eq!(result.unwrap(),
            vec![
            Triple(Resource::make_uri("foo"), NamedNode::make_uri("bar"), Value::make_uri("a")),
            Triple(Resource::make_uri("foo"), NamedNode::make_uri("b"), Value::make_uri("c")),
            Triple(Resource::make_uri("foo"), NamedNode::make_uri("b"), Value::make_uri("d"))
        ]);
    }

    #[test]
    fn test_a() {
        let result = parse_turtle("<foo> a <bar> .");
        assert_eq!(result.unwrap(),
                   vec![
            Triple(Resource::make_uri("foo"), NamedNode::make_uri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"), Value::make_uri("bar"))
            ]);

    }

    #[test]
    fn test_bnodes() {
        //let result = parse_turtle("<foo> <bar> ( <foo> <bar> ) .");
        let result = parse_turtle("<foo> <bar> _:test , [ ] , [ <foo> <bar> ] , ( <foo> <bar> ) .");
        assert_eq!(result.unwrap(), vec![
            Triple(Resource::make_blank("nodeID1"), NamedNode::make_uri("foo"), Value::make_uri("bar")),
            Triple(Resource::make_blank("nodeID2"), NamedNode::make_uri("http://www.w3.org/1999/02/22-rdf-syntax-ns#first"), Value::make_uri("foo")),
            Triple(Resource::make_blank("nodeID2"), NamedNode::make_uri("http://www.w3.org/1999/02/22-rdf-syntax-ns#rest"), Value::make_blank("nodeID3")),
            Triple(Resource::make_blank("nodeID3"), NamedNode::make_uri("http://www.w3.org/1999/02/22-rdf-syntax-ns#first"), Value::make_uri("bar")),
            Triple(Resource::make_blank("nodeID3"), NamedNode::make_uri("http://www.w3.org/1999/02/22-rdf-syntax-ns#rest"), Value::make_uri("http://www.w3.org/1999/02/22-rdf-syntax-ns#nil")),
            Triple(Resource::make_uri("foo"), NamedNode::make_uri("bar"), Value::make_blank("test")),
            Triple(Resource::make_uri("foo"), NamedNode::make_uri("bar"), Value::make_blank("nodeID0")),
            Triple(Resource::make_uri("foo"), NamedNode::make_uri("bar"), Value::make_blank("nodeID1")),
            Triple(Resource::make_uri("foo"), NamedNode::make_uri("bar"), Value::make_blank("nodeID2"))
            ]);

    }

    #[test]
    fn test_bnode2() {
        let result = parse_turtle("<foo> <bar> ( ) .");
        assert_eq!(result.unwrap(), vec![
            Triple(Resource::make_uri("foo"), NamedNode::make_uri("bar"), Value::make_uri("http://www.w3.org/1999/02/22-rdf-syntax-ns#nil"))
            ]);
    }

    #[test]
    fn test_urls() {
        let result = parse_turtle("<> <#bar> <../foo> .");
        assert_eq!(result.unwrap(), vec![
            Triple(Resource::make_uri(""), NamedNode::make_uri("#bar"), Value::make_uri("../foo"))
        ]);

    }

    #[test]
    fn test_longer() {
        let data = "@base <http://example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix foaf: <http://xmlns.com/foaf/0.1/> .
@prefix rel: <http://www.perceive.net/schemas/relationship/> .

<#green-goblin>
    a foaf:Person ;    # in the context of the Marvel universe
    rel:enemyOf <#spiderman> ;
    foaf:name \"Green Goblin\" .

<#spiderman>
    rel:enemyOf <#green-goblin> ;
    a foaf:Person ;
    foaf:name \"Spiderman\", \"Человек-паук\"@ru .";
        eprintln!("{:?}", parse_turtle(data).unwrap());
    }
}
