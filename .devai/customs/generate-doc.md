# Data

```rhai
let paths = file::list("./src/**/*.rs");
let files = [];

for f in paths {
    let content = file::load(f.path);
    files.push(content);
}

return #{files: files};
```

# Instruction

- Below is a list of rust file content in this project.
- Please document, summarise and consolidate them into one concise and clear developer documentation in markdown.
- Make sure you use markdown heading for each sections.

{{#each data.files}}

```rust
{{this.content}}
```

{{/each}}

# Output

```rhai
let content = ai_output;
let path = "./doc/README.md";
file::save(path, content);
return "documentation generated at: " + path
```
