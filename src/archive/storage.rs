use tantivy::{
    DateTime, Directory, Index, IndexWriter, ReloadPolicy, Score, collector::TopDocs, doc,
    query::QueryParser, schema::*, time::OffsetDateTime,
};
use uuid::Uuid;

pub struct MemoryStorage {
    schema: Schema,
    index: Index,
}

impl MemoryStorage {
    pub fn new() -> Self {
        // Define schema
        let mut schema_builder = Schema::builder();
        let opts = DateOptions::from(INDEXED)
            .set_stored()
            .set_fast()
            .set_precision(tantivy::schema::DateTimePrecision::Seconds);

        let id = schema_builder.add_text_field("id", STRING | STORED);
        let content = schema_builder.add_text_field("content", TEXT | STORED);
        let timestamp = schema_builder.add_date_field("timestamp", INDEXED | STORED);
        let embedding = schema_builder.add_facet_field("embedding", STORED); // For future embedding support

        let schema = schema_builder.build();

        // Create index in memory (for persistence, use a directory)
        let index = Index::create_in_ram(schema.clone());
        // let dir = Directory::open_write(&self, path)
        // let index = Index::open_or_create(Directory::, schema.clone()).unwrap();

        MemoryStorage { schema, index }
    }

    pub fn add_memory(&self, text: &str) -> tantivy::Result<()> {
        let mut writer: IndexWriter = self.index.writer(50_000_000)?;

        let id = self.schema.get_field("id").unwrap();
        let content = self.schema.get_field("content").unwrap();
        let timestamp = self.schema.get_field("timestamp").unwrap();

        let uuid = Uuid::new_v4().to_string();

        // writer.add_document(doc!(
        //     id => uuid,
        //     content => text,
        //     timestamp => now
        // ))?;

        let mut doc = TantivyDocument::default();

        doc.add_text(id, uuid);
        doc.add_text(content, text);

        let now = DateTime::from_utc(OffsetDateTime::now_utc());
        doc.add_date(timestamp, now);

        writer.add_document(doc)?;

        writer.commit()?;
        Ok(())
    }

    pub fn search(&self, query: &str, threshold: Score) -> tantivy::Result<Vec<String>> {
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        let searcher = reader.searcher();

        let content = self.schema.get_field("content").unwrap();
        let query_parser = QueryParser::for_index(&self.index, vec![content]);
        let parsed_query = query_parser.parse_query(query)?;

        let top_docs = searcher.search(&parsed_query, &TopDocs::with_limit(10))?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            if score >= threshold {
                let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;
                if let Some(content) = retrieved_doc.get_first(content) {
                    if let Some(text) = content.as_str() {
                        println!("retrieved {} with score {}", text, score);
                        results.push(text.to_string());
                    }
                }
            }
        }

        Ok(results)
    }
}
