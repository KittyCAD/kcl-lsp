# kcl-lsp

The `kcl` [Language Server Protocol](https://microsoft.github.io/language-server-protocol)
implementation.

### Syntax Highlighting

Make sure your semantic token is enabled, you could enable your `semantic token` by
adding this line to your `settings.json`:

```json
{
  "editor.semanticHighlighting.enabled": true
}
```

We automatically set this up as the default for the plugin so this should work
out of the box.

### Development

```bash
$ yarn install
$ cargo build
```

- press <kbd>F5</kbd> or change to the Debug panel and click <kbd>Launch Client</kbd>

> **Note**
>
> If encountered errors like `Cannot find module '/xxx/xxx/dist/extension.js'`
> please try run command `tsc -b` manually
