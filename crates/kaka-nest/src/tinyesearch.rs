use tinysearch::{BasicPost, Post, TinySearch};

use crate::build_page::{BlogPost, PostList};

impl Post for BlogPost {
    fn title(&self) -> &str {
        &self.title
    }

    fn url(&self) -> &str {
        &self.url
    }

    fn body(&self) -> Option<&str> {
        Some(&self.contents)
    }

    fn meta(&self) -> Option<&str> {
        None
    }
}

impl PostList {
    pub fn create_tinysearch_index(&self) -> Result<(), Box<dyn std::error::Error>> {
        let tinyesearch = TinySearch::new();
        let index = tinyesearch.build_index(self.blogs.clone())?;

        let result = tinyesearch.search(&index, "rust", 5);
        dbg!(result);
        Ok(())
    }
}
