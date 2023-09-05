# kcl-lsp

The `kcl` [Language Server Protocol](https://microsoft.github.io/language-server-protocol)
implementation.

## VSCode

Install our extension: [KittyCAD Language Server](https://marketplace.visualstudio.com/items?itemName=KittyCAD.kcl-language-server)

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

## Neovim

You can add the following to your `vim` configuration if you are using `lspconfig`.

This is [@jessfraz's
setup](https://github.com/jessfraz/.vim/blob/master/vimrc#L935).

```vim
if executable('kcl-language-server')
lua << EOF
local lspconfig = require 'lspconfig'
local configs = require 'lspconfig.configs'

if not configs.kcl_lsp then
  configs.kcl_lsp = {
    default_config = {
      cmd = {'kcl-language-server', 'server', '--stdio'},
      filetypes = {'kcl'},
      root_dir = lspconfig.util.root_pattern('.git'),
    },
    docs = {
      description = [=[
https://github.com/KittyCAD/kcl-lsp
https://kittycad.io

The KittyCAD Language Server Protocol implementation for the KCL language.

To better detect kcl files, the following can be added:


    vim.cmd [[ autocmd BufRead,BufNewFile *.kcl set filetype=kcl ]]

]=],
      default_config = {
        root_dir = [[root_pattern(".git")]],
      },
    }
  }
end

lspconfig.kcl_lsp.setup{}
EOF
else
  echo "You might want to install kcl-language-server: https://github.com/KittyCAD/kcl-lsp/releases"
end
```

## Development

```bash
$ yarn install
$ cargo build
```

- press <kbd>F5</kbd> or change to the Debug panel and click <kbd>Launch Client</kbd>

> **Note**
>
> If encountered errors like `Cannot find module '/xxx/xxx/dist/extension.js'`
> please try run command `tsc -b` manually
