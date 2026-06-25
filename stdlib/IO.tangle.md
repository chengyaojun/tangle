# IO

### IOUtils

#### 读文件 (readFile)
* `path`: file path (String)

```@tangle
return fs.readFileSync(path, "utf-8")
```

#### 写文件 (writeFile)
* `path`: file path (String)
* `content`: content (String)

```@tangle
fs.writeFileSync(path, content, "utf-8")
return true
```
