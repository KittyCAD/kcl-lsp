# kcl-lsp

The `kcl` [Language Server Protocol](https://microsoft.github.io/language-server-protocol)
implementation.

This is just the server process. Most of the actual code for the server endpoints is in the same repo as the AST/tokenizer/parser since it's all relevant. That is [modeling-app](https://github.com/kittycad/modeling-app).

## VSCode

Install our extension: [KittyCAD Language Server](https://marketplace.visualstudio.com/items?itemName=KittyCAD.kcl-language-server)

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
      single_file_support = true,
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

## Helix

Add this to your `languages.toml` file. Remember to change ``/Users/adamchalmers` to your path.

Note that we don't currently have Treesitter parsers, so there won't be syntax highlighting.

```toml
[[language]]
name = "kcl"
scope = "source.kcl"
injection-regex = "kcl"
file-types = ["kcl"]
comment-tokens = "//"
indent = { tab-width = 2, unit = "  " }
language-servers = [ "kcl-lsp" ]
block-comment-tokens = { start = "/*", end = "*/"}


[language-server.kcl-lsp]
command = "/Users/adamchalmers/kc-repos/kcl-lsp/target/release/kcl-language-server"
args = ["server", "--stdio"]
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
