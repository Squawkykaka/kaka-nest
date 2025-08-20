---
date: 2025-08-16
published: true
read_mins: 5
tags:
  - "#rust"
  - "#blog-software"
  - "#database"
---
Whenever a file gets changed, the files contents gets hashed and the change gets pushed into the database.

## computing html
parsed sections of code get placed into the database, e.g. each codeblock and section, omly regenerating bits needing regernerating
Possible problems
- what if a template changes? what do i do then

hash revision, all files *changed* for a specific version
convert current 