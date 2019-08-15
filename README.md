ELEXIS Dictionary Service
-------------------------


This tool provides a simple way to host dictionaries that can be contributed 
to the ELEXIS infrastructure. This interface is the refrence implementation of
the REST API defined here:

https://elexis-eu.github.io/elexis-rest

Installation
------------

### From Source

This tool can be built with [Rust/Cargo](https://doc.rust-lang.org/cargo/getting-started/index.html) using the following command

    cargo build --release

This will create a single binary at `target/release/elexis-dictionary-service`.

### By Docker

TODO

Usage
-----

The ELEXIS dictionary service supports a number of commands

### Loading data

Data can be loaded with the `load` command

```
USAGE:
    elexis-dictionary-service load [FLAGS] [OPTIONS] <data>

FLAGS:
    -h, --help       Prints help information
        --no-sql     Do not use SQLite (all data is temporary and session only)
    -V, --version    Prints version information

OPTIONS:
        --db-path <db_path>                                  The path to use for the database (Default: eds.db)
    -f, --format <json|ttl|tei>                              The format of the input
        --genre <gen|lrn|ety|spe|his|ort|trm>                The genre(s) of the dataset (comma separated)
        --id <id>                                            The identifier of the dataset
        --release <PUBLIC|NONCOMMERCIAL|RESEARCH|PRIVATE>    The release level of the resource

ARGS:
    <data>    The data to host
```

For example to load a file it is normally sufficient to give a command as follows:

```sh
# A Json file
elexis-dictionary-service load example/example.json
# A TEI-Lex0 file
elexis-dictionary-service load example/example-tei.xml --id tei_dict --release PUBLIC
# An OntoLex file
elexis-dictionary-service load example/example.rdf --release PUBLIC
```

### Starting the server

The REST server may be started with the `start` command:

```
Start the server

USAGE:
    elexis-dictionary-service start [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
        --no-sql     Do not use SQLite (all data is temporary and session only)
    -V, --version    Prints version information

OPTIONS:
    -d, --data <data>                                        Also load a single data file
        --db-path <db_path>                                  The path to use for the database (Default: eds.db)
    -f, --format <json|ttl|tei>                              The format of the input
        --genre <gen|lrn|ety|spe|his|ort|trm>                The genre(s) of the dataset (comma separated)
        --id <id>                                            The identifier of the dataset
    -p, --port <port>                                        The port to start the server on
        --release <PUBLIC|NONCOMMERCIAL|RESEARCH|PRIVATE>    The release level of the resource
```

For example to start a server

```sh
elexis-dictionary-service start
```

The server will be available at http://localhost:8000/

To start a temporary server for a single file (not using SQlite) the following command can be used

```sh
elexis-dictionary-service start -d example/example.json --no-sql
```

### Deleting a dictionary

A dictionary may be removed from the server with the `delete` command

```
USAGE:
    elexis-dictionary-service delete [OPTIONS] [data]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --db-path <db_path>    The path to use for the database (Default: eds.db)

ARGS:
    <data>    Also load a single data file
```

For example

```sh
elexis-dictionary-servcie delete dict_id
```

Formats
-------

### Json

The Json format consists of an object of the following form

```json
{
    "dict_id": {
        "meta": {    },
        "entries: [    ]
    }
}
```

Where `dict_id` is the name of the dictionary, the `meta` value is exactly
as would be returned by the [about](https://elexis-eu.github.io/elexis-rest/#operation/about)
REST call. The `entries` value is an array where each element
is as would be returned by the [entry as Json](https://elexis-eu.github.io/elexis-rest/#operation/getEntryById) REST call

### TEI-Lex0

The TEI-Lex0 document should be a valid XML document with at least the following tags

```xml
<TEI xmlns="http://www.tei-c.org/ns/1.0">
    <teiHeader>
        <fileDesc>
            <titleStmt>
                <title>Name of the dictionary</author>
            </titleStmt>
            <publicationStmt>
                <publisher>Named of the publisher</publisher>
                <availability>
                    <licence target="http://url.of.licence">...</licence>
                </availability>
            </publicationStmt>
            <sourceDesc>
                <author>Name of the author</author>
            </sourceDesc>
        </fileDesc>
    </teiHeader>
    <body>
       <entry xml:lang="en" xml:id="test">
        <form type="lemma">
            <orth>girl</orth>
        </form>
        <form type="variant">
            <orth>girls</orth>
        </form>
        <gramGrp>
            <gram type="pos" norm="NOUN">noun</gram>
        </gramGrp>
        <senses>
            <sense>
                <def>young female</def>#
            </sense>
        </senses>
    </body>
</TEI>
```

The following constraints are required

1. A `licence` must be given with at `target`
2. An `entry` must have a `form[@type=lemma]`
3. An `entry` must have a `gram[@type=pos]` and it should have a `norm` referring
to a UD category unless mapping is used (see below)
4. An `entry` must have a `lang` and a `id`
5. An `entry` must not occur within another entry


### OntoLex

An OntoLex document should be a valid [Turtle](https://www.w3.org/TR/turtle/) 
document such as follows:

```
@prefix lime: <http://www.w3.org/ns/lemon/lime#> .
@prefix ontolex: <http://www.w3.org/ns/lemon/ontolex#> .
@prefix dct: <http://purl.org/dc/terms/> .
@prefix foaf: <http://xmlns.com/foaf/0.1/> .
@prefix skos: <http://www.w3.org/2004/02/skos/core#> .
@prefix lexinfo: <http://www.lexinfo.net/ontology/2.0/lexinfo#> .

<#dictionary> a lime:Lexicon ;
    lime:language "en" ;
    dct:license <http://www.example.com/license> ;
    dct:description "A test resource" ;
    dct:creator [
        foaf:name "Joe Bloggs" ;
        foaf:mbox <mailto:test@example.com> ;
        foaf:homepage <http://www.example.com/>
    ] ;
    dct:publisher [
        foaf:name "Publisher"
    ] ;
    lime:entry <#entry1>, <#test> .

<#entry1> a ontolex:LexicalEntry ;
    lexinfo:partOfSpeech lexinfo:commonNoun ;
    ontolex:canonicalForm [
        ontolex:writtenRep "cat"@en 
    ] ;
    ontolex:sense [
        skos:definition "This is a definition"@en
    ] .

<#test>  a ontolex:LexicalEntry ;
    ontolex:canonicalForm [
        ontolex:writtenRep "dog"@en 
    ] ;
    ontolex:sense [
        ontolex:reference <http://www.example.com/ontology>  
    ] .
```

In order to process the file well, certain information should be grouped together,
in particular all information about the lexicon should follow after the triple

```
<#dictionary> a lime:Lexicon
```

A dictionary must have a `lime:language` and a `dct:license`.


The entry starts with a triple of the form

```
<#entry1> a ontolex:LexicalEntry
```

All triples after this until another similar triple occurs in the file are
considered the description of this entry. 

All entries must have `ontolex:canonicalForm` with an `ontolex:writtenRep`. 

All entries must be given by URIs and referred to by a `lime:entry` triple from
a lexicon
