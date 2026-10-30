# Neovim keybinding inventory

Generated from current installed configuration; remote was not fetched.

## Counts

- runtime_unique: 619
- builtin_grammar_entries: 72
- observed_portable: 16
- observed_unmapped: 603
- context_variant_groups: 37
- inactive_templates: ['lua/community.lua', 'lua/plugins/astrolsp.lua', 'lua/plugins/astroui.lua', 'lua/plugins/mason.lua', 'lua/plugins/none-ls.lua', 'lua/plugins/treesitter.lua', 'lua/plugins/user.lua']

## Verification limits

- Both headless and real PTY TUI runs: startup + VeryLazy + files + InsertEnter + LSP attach; no interactive terminal buffer, DAP session, OpenCode service or marimo session started.
- PDF fixture is not a valid PDF; extension hooks only, not interactive PDF plugin behavior. Its retained Rust filetype/client is evidence, not PDF LSP support.
- Visual Line/Block use x/v mapping namespaces, not independent nvim_get_keymap modes.
- Select/operator/terminal maps collected; entering all dynamic plugin states is not proven.
- lua/community.lua returns {}; no lua/community directory exists. Installed plugins are not evidence of active community imports.
- lhsraw binary termcodes were decoded with replacement for JSON transport; human-readable lhs is preserved.

## Runtime and builtin registry

| Keys | Mode | Description | Plugin | Scope | Observed contexts | LeafView |
|---|---|---|---|---|---|---|
| <NL> | t | Terminal down window navigation | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-_> | t | jupynvim: terminal toggle (remote when SSH-connected) | jupynvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-/> | t | jupynvim: terminal toggle (remote when SSH-connected) | jupynvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <F7> | t | Toggle terminal | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-K> | t | Terminal up window navigation | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-H> | t | Terminal left window navigation | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-L> | t | Terminal right window navigation | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-'> | t | Toggle terminal | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Tab> | x | Indent line | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space dE | x | Evaluate Input | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space go | x | Git browse (open) | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space / | x | Toggle comment | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space os | x | Lua callback /home/yaroslav/.local/share/nvim/lazy/lazy.nvim/lua/lazy/core/handler/keys.lua:121 | core | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space oa | x | Lua callback /home/yaroslav/.local/share/nvim/lazy/lazy.nvim/lua/lazy/core/handler/keys.lua:121 | core | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| # | x | :help v_#-default | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| % | x | <Plug>(MatchitVisualForward) | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| * | x | :help v_star-default | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <lt> | x | <lt>gv | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| > | x | >gv | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| @ | x | :help v_@-default | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Q | x | :help v_Q-default | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| [% | x | <Plug>(MatchitVisualMultiBackward) | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ]% | x | <Plug>(MatchitVisualMultiForward) | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| a% | x | <Plug>(MatchitVisualTextObject) | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| g% | x | <Plug>(MatchitVisualBackward) | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| go | x | Lua callback /home/yaroslav/.local/share/nvim/lazy/lazy.nvim/lua/lazy/core/handler/keys.lua:121 | core | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| gra | x | vim.lsp.buf.code_action() | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| gc | x | Toggle comment | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| gx | x | Opens filepath or URI under cursor with the system handler (file explorer, web browser, …) | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| j | x | Move cursor down | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| k | x | Move cursor up | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>(MatchitVisualTextObject) | x | <Plug>(MatchitVisualMultiBackward)o<Plug>(MatchitVisualMultiForward) | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>(MatchitVisualMultiForward) | x | :<C-U>call matchit#MultiMatch("W",  "n")<CR>m'gv`` | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>(MatchitVisualMultiBackward) | x | :<C-U>call matchit#MultiMatch("bW", "n")<CR>m'gv`` | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>(MatchitVisualBackward) | x | :<C-U>call matchit#Match_wrapper('',0,'v')<CR>m'gv`` | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>(MatchitVisualForward) | x | :<C-U>call matchit#Match_wrapper('',1,'v')<CR>:if col("''") != col("$") \| exe ":normal! m'" \| endif<CR>gv`` | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <S-Tab> | x | Unindent line | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <NL> | n | Move to below split | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space sg | n | jupynvim: grep (remote when SSH-connected) | jupynvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space   | n | jupynvim: files (remote when SSH-connected) | jupynvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space tr | n | jupynvim: toggle second remote terminal (right) | jupynvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space E | n | jupynvim: explorer (remote when SSH-connected) | jupynvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space fT | n | Find TODOs | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space th | n | ToggleTerm horizontal split | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space dq | n | Close Session | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space tp | n | ToggleTerm python | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space bd | n | Close buffer from tabline | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space tf | n | ToggleTerm float | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space b\ | n | Horizontal split buffer from tabline | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space dB | n | Clear Breakpoints | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space b\| | n | Vertical split buffer from tabline | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space uR | n | Toggle reference highlighting (global) | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space db | n | Toggle Breakpoint (F9) | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space tv | n | ToggleTerm vertical split | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space ur | n | Toggle reference highlighting (buffer) | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space gg | n | ToggleTerm lazygit | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space fF | n | Find all files | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space fg | n | Find git files | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space uz | n | Toggle color highlight | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space fh | n | Find help | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space fk | n | Find keymaps | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space fm | n | Find man | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space fn | n | Find notifications | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space fO | n | Find old files (cwd) | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space fu | n | Find undo history | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space fw | n | Find words | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space fr | n | Find registers | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | files.recent |
| Space fs | n | Find buffers/recent/files | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space ff | n | jupynvim: files (remote when SSH-connected) | jupynvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | files.find |
| Space ft | n | Find themes | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space fC | n | Find commands | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space uw | n | Toggle wrap | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space f<CR> | n | Resume previous search | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space fW | n | Find words in all files | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space gC | n | Git commits (current file) | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space ld | n | Hover diagnostics | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | lsp.diagnostics |
| Space lD | n | Search diagnostics | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space S. | n | Load current dirsession | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space Sl | n | Load last session | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space ls | n | Search symbols | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space SF | n | Load a dirsession | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space uZ | n | Toggle zen mode | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space bb | n | Select buffer from tabline | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space Sf | n | Load a session | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space h | n | Home Screen | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space u\| | n | Toggle indent guides | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space uD | n | Dismiss notifications | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space dr | n | Restart (C-F5) | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space pM | n | Mason Update | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space gb | n | Git branches | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space gc | n | Git commits (repository) | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space Sd | n | Delete a session | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space w | n | Save | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | editor.save |
| Space gt | n | Git status | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space q | n | Quit Window | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space St | n | Save this tab's session | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space gT | n | Git stash | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space Q | n | Exit AstroNvim | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space SS | n | Save this dirsession | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space n | n | New File | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space dh | n | Debugger Hover | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space fa | n | Find AstroNvim config files | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space Ss | n | Save this session | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space fb | n | Find buffers | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space uv | n | Toggle virtual text | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space fc | n | Find word under cursor | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space pm | n | Mason Installer | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space uu | n | Toggle URL highlight | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space ut | n | Toggle tabline | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space uS | n | Toggle conceal | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space R | n | Rename file | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space dC | n | Conditional Breakpoint (S-F9) | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space us | n | Toggle spellcheck | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space / | n | jupynvim: grep (remote when SSH-connected) | jupynvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space go | n | Git browse (open) | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space up | n | Toggle paste mode | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space uc | n | Toggle autocompletion (buffer) | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space br | n | Close all buffers to the right | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space uC | n | Toggle autocompletion (global) | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space uN | n | Toggle Notifications | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space uy | n | Toggle syntax highlight (buffer) | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space pi | n | Plugins Install | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space un | n | Change line numbering | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space uA | n | Toggle rooter autochdir | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space uV | n | Toggle virtual lines | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space ps | n | Plugins Status | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space ul | n | Toggle statusline | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space ub | n | Toggle background | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space pS | n | Plugins Sync | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space ua | n | Toggle autopairs | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space ud | n | Toggle diagnostics | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space du | n | Toggle Debugger UI | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space pu | n | Plugins Check Updates | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space bc | n | Close all buffers except current | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space ug | n | Toggle signcolumn | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space ui | n | Change indent setting | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space pU | n | Plugins Update | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space bl | n | Close all buffers to the left | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space u> | n | Toggle foldcolumn | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space bsm | n | By modification | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space pa | n | Update Lazy and Mason | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space bsi | n | By buffer number | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space c | n | Close buffer | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | editor.close |
| Space dR | n | Toggle REPL | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space SD | n | Delete a dirsession | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space C | n | Force close buffer | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space bse | n | By extension | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space bsr | n | By relative path | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space bsp | n | By full path | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space dE | n | Evaluate Input | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space di | n | Step Into (F11) | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space bC | n | Close all buffers | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space bp | n | Previous buffer | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | editor.previous_buffer |
| Space dQ | n | Terminate Session (S-F5) | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space f' | n | Find marks | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space fl | n | Find lines | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space fo | n | Find old files | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space dc | n | Start/Continue (F5) | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space fp | n | Find projects | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space do | n | Step Over (F10) | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space dO | n | Step Out (S-F11) | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space xq | n | Quickfix List | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space tl | n | ToggleTerm lazygit | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space tn | n | ToggleTerm node | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space dp | n | Pause (F6) | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space xl | n | Location List | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space ds | n | Run To Cursor | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space e | n | jupynvim: explorer (remote when SSH-connected) | jupynvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | explorer.toggle |
| Space o | n | Открыть структуру файла | core | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space ip | n | Paste clipboard image | core | global | startup, rust, pdf, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python | not mapped / verify grammar |
| Space ma | n | Marimo Attach | core | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space mi | n | Marimo Status | core | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space mf | n | Marimo Follow | core | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space mR | n | Marimo Run All | core | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space mr | n | Marimo Run Cell | core | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space mq | n | Marimo Stop | core | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space ms | n | Marimo Start | core | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space md | n | Marimo Detach | core | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space oi | n | Lua callback /home/yaroslav/.local/share/nvim/lazy/lazy.nvim/lua/lazy/core/handler/keys.lua:121 | core | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space or | n | Lua callback /home/yaroslav/.local/share/nvim/lazy/lazy.nvim/lua/lazy/core/handler/keys.lua:121 | core | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space ou | n | Lua callback /home/yaroslav/.local/share/nvim/lazy/lazy.nvim/lua/lazy/core/handler/keys.lua:121 | core | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space on | n | Lua callback /home/yaroslav/.local/share/nvim/lazy/lazy.nvim/lua/lazy/core/handler/keys.lua:121 | core | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space os | n | Lua callback /home/yaroslav/.local/share/nvim/lazy/lazy.nvim/lua/lazy/core/handler/keys.lua:121 | core | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space oa | n | Lua callback /home/yaroslav/.local/share/nvim/lazy/lazy.nvim/lua/lazy/core/handler/keys.lua:121 | core | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| % | n | <Plug>(MatchitNormalForward) | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| & | n | :help &-default | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ,v | n | Select Python environment | core | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <lt>b | n | Move buffer tab left | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| >b | n | Move buffer tab right | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| K | n | Показать документацию LSP | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | lsp.hover |
| Y | n | :help Y-default | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| [% | n | <Plug>(MatchitNormalMultiBackward) | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| [e | n | Previous error | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| [w | n | Previous warning | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| [r | n | Previous reference | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| [  | n | Add empty line above cursor | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| [B | n | :brewind | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| [b | n | Previous buffer | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | editor.previous_buffer |
| [<C-T> | n |  :ptprevious | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| [T | n | Previous TODO comment | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| [t | n | Previous tab | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| [A | n | :rewind | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| [a | n | :previous | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| [<C-L> | n | :lpfile | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| [L | n | :lrewind | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| [l | n | :lprevious | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| [<C-Q> | n | :cpfile | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| [Q | n | :crewind | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| [q | n | :cprevious | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| [D | n | Jump to the first diagnostic in the current buffer | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| [d | n | Jump to the previous diagnostic in the current buffer | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| \ | n | Horizontal Split | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ]% | n | <Plug>(MatchitNormalMultiForward) | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ]w | n | Next warning | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ]e | n | Next error | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ]r | n | Next reference | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ]  | n | Add empty line below cursor | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ]B | n | :blast | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ]b | n | Next buffer | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | editor.next_buffer |
| ]<C-T> | n | :ptnext | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ]T | n | Next TODO comment | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ]t | n | Next tab | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ]A | n | :last | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ]a | n | :next | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ]<C-L> | n | :lnfile | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ]L | n | :llast | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ]l | n | :lnext | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ]<C-Q> | n | :cnfile | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ]Q | n | :clast | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ]q | n | :cnext | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ]D | n | Jump to the last diagnostic in the current buffer | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ]d | n | Jump to the next diagnostic in the current buffer | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| g% | n | <Plug>(MatchitNormalBackward) | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| gl | n | Hover diagnostics | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| gco | n | Add Comment Below | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| gcO | n | Add Comment Above | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| go | n | Lua callback /home/yaroslav/.local/share/nvim/lazy/lazy.nvim/lua/lazy/core/handler/keys.lua:121 | core | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| goo | n | Lua callback /home/yaroslav/.local/share/nvim/lazy/lazy.nvim/lua/lazy/core/handler/keys.lua:121 | core | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| gO | n | vim.lsp.buf.document_symbol() | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| grt | n | vim.lsp.buf.type_definition() | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| gri | n | vim.lsp.buf.implementation() | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| grr | n | vim.lsp.buf.references() | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| gra | n | vim.lsp.buf.code_action() | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| grn | n | vim.lsp.buf.rename() | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| gcc | n | Toggle comment line | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| gc | n | Toggle comment | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| gx | n | Opens filepath or URI under cursor with the system handler (file explorer, web browser, …) | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| j | n | Move cursor down | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| k | n | Move cursor up | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| \| | n | Vertical Split | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>(MatchitNormalMultiForward) | n | :<C-U>call matchit#MultiMatch("W",  "n")<CR> | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>(MatchitNormalMultiBackward) | n | :<C-U>call matchit#MultiMatch("bW", "n")<CR> | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>(MatchitNormalBackward) | n | :<C-U>call matchit#Match_wrapper('',0,'n')<CR> | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>(MatchitNormalForward) | n | :<C-U>call matchit#Match_wrapper('',1,'n')<CR> | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-_> | n | jupynvim: terminal toggle (remote when SSH-connected) | jupynvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-/> | n | jupynvim: terminal toggle (remote when SSH-connected) | jupynvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-Left> | n | Resize split left | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-Down> | n | Resize split down | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-Right> | n | Resize split right | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-K> | n | Move to above split | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-Up> | n | Resize split up | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-'> | n | Toggle terminal | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <F10> | n | Debugger: Step Over | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <F7> | n | Toggle terminal | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <F29> | n | Debugger: Restart | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <F9> | n | Debugger: Toggle Breakpoint | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-H> | n | Move to left split | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-S> | n | Force write | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-Q> | n | Force quit | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <F5> | n | Debugger: Start | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <F17> | n | Debugger: Stop | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <F21> | n | Debugger: Conditional Breakpoint | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <F6> | n | Debugger: Pause | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <F11> | n | Debugger: Step Into | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <F23> | n | Debugger: Step Out | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <M-S> | n | Capture selected screen area | core | global | startup, rust, pdf, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python | not mapped / verify grammar |
| <C-W><C-D> | n | Show diagnostics under the cursor | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-W>d | n | Show diagnostics under the cursor | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-L> | n | Move to right split | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| % | o | <Plug>(MatchitOperationForward) | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| [% | o | <Plug>(MatchitOperationMultiBackward) | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ]% | o | <Plug>(MatchitOperationMultiForward) | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| g% | o | <Plug>(MatchitOperationBackward) | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| gc | o | Comment textobject | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>(MatchitOperationMultiForward) | o | :<C-U>call matchit#MultiMatch("W",  "o")<CR> | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>(MatchitOperationMultiBackward) | o | :<C-U>call matchit#MultiMatch("bW", "o")<CR> | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>(MatchitOperationBackward) | o | :<C-U>call matchit#Match_wrapper('',0,'o')<CR> | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>(MatchitOperationForward) | o | :<C-U>call matchit#Match_wrapper('',1,'o')<CR> | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Tab> | v | Indent line | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space dE | v | Evaluate Input | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space go | v | Git browse (open) | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space / | v | Toggle comment | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space os | v | Lua callback /home/yaroslav/.local/share/nvim/lazy/lazy.nvim/lua/lazy/core/handler/keys.lua:121 | core | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space oa | v | Lua callback /home/yaroslav/.local/share/nvim/lazy/lazy.nvim/lua/lazy/core/handler/keys.lua:121 | core | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| # | v | :help v_#-default | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| % | v | <Plug>(MatchitVisualForward) | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| * | v | :help v_star-default | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <lt> | v | <lt>gv | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| > | v | >gv | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| @ | v | :help v_@-default | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Q | v | :help v_Q-default | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| [% | v | <Plug>(MatchitVisualMultiBackward) | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ]% | v | <Plug>(MatchitVisualMultiForward) | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| a% | v | <Plug>(MatchitVisualTextObject) | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| g% | v | <Plug>(MatchitVisualBackward) | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| go | v | Lua callback /home/yaroslav/.local/share/nvim/lazy/lazy.nvim/lua/lazy/core/handler/keys.lua:121 | core | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| gra | v | vim.lsp.buf.code_action() | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| gc | v | Toggle comment | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| gx | v | Opens filepath or URI under cursor with the system handler (file explorer, web browser, …) | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| j | v | Move cursor down | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| k | v | Move cursor up | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>(MatchitVisualTextObject) | v | <Plug>(MatchitVisualMultiBackward)o<Plug>(MatchitVisualMultiForward) | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>(MatchitVisualMultiForward) | v | :<C-U>call matchit#MultiMatch("W",  "n")<CR>m'gv`` | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>(MatchitVisualMultiBackward) | v | :<C-U>call matchit#MultiMatch("bW", "n")<CR>m'gv`` | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>(MatchitVisualBackward) | v | :<C-U>call matchit#Match_wrapper('',0,'v')<CR>m'gv`` | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>(MatchitVisualForward) | v | :<C-U>call matchit#Match_wrapper('',1,'v')<CR>:if col("''") != col("$") \| exe ":normal! m'" \| endif<CR>gv`` | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <S-Tab> | v | Unindent line | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-S> | v | vim.lsp.buf.signature_help() | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>(TelescopeFuzzyCommandSearch) | c | <C-\>e "lua require('telescope.builtin').command_history { default_text = [=[" . escape(getcmdline(), '"') . "]=] }"<CR><CR> | telescope.nvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <F7> | i | Toggle terminal | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-'> | i | Toggle terminal | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <M-S> | i | Capture selected screen area | core | global | startup, rust, pdf, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python | not mapped / verify grammar |
| <S-Tab> | i | vim.snippet.jump if active, otherwise <S-Tab> | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-S> | i | vim.lsp.buf.signature_help() | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-W> | i | :help i_CTRL-W-default | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-U> | i | :help i_CTRL-U-default | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Tab> | i | vim.snippet.jump if active, otherwise <Tab> | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space ip | i | Paste clipboard image | core | global | startup, rust, pdf, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python | not mapped / verify grammar |
| <Tab> | s | Indent line | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space dE | s | Evaluate Input | AstroNvim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <lt> | s | <lt>gv | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| > | s | >gv | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <S-Tab> | s | Unindent line | user | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-S> | s | vim.lsp.buf.signature_help() | Neovim | global | startup, rust, pdf, typst, python, config, markdown, tui:startup, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-expand-repeat | x | LuaSnip: Repeat last node expansion | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <M-i> | x | Lua callback /home/yaroslav/.local/share/nvim/lazy/vim-illuminate/lua/illuminate.lua:301 | vim-illuminate | global | rust, pdf, typst, python, config, markdown, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-expand-repeat | n | LuaSnip: Repeat last node expansion | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-delete-check | n | LuaSnip: Removes current snippet from jumplist | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <M-p> | n | Move to previous reference | vim-illuminate | global | rust, pdf, typst, python, config, markdown, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <M-n> | n | Move to next reference | vim-illuminate | global | rust, pdf, typst, python, config, markdown, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-expand-repeat | o | LuaSnip: Repeat last node expansion | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <M-i> | o | Lua callback /home/yaroslav/.local/share/nvim/lazy/vim-illuminate/lua/illuminate.lua:301 | vim-illuminate | global | rust, pdf, typst, python, config, markdown, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-jump-prev | v | LuaSnip: Jump to the previous node | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-jump-next | v | LuaSnip: Jump to the next node | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-prev-choice | v | LuaSnip: Change to the previous choice from the choiceNode | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-next-choice | v | LuaSnip: Change to the next choice from the choiceNode | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-expand-snippet | v | LuaSnip: Expand the current snippet | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-expand-or-jump | v | LuaSnip: Expand or jump in the current snippet | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-expand-repeat | v | LuaSnip: Repeat last node expansion | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <M-i> | v | Lua callback /home/yaroslav/.local/share/nvim/lazy/vim-illuminate/lua/illuminate.lua:301 | vim-illuminate | global | rust, pdf, typst, python, config, markdown, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-expand-repeat | c | LuaSnip: Repeat last node expansion | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-delete-check | c | LuaSnip: Removes current snippet from jumplist | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <S-Tab> | c | blink.cmp: <Custom Fn>, Select Prev | blink.cmp | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-Y> | c | blink.cmp: Select And Accept | blink.cmp | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Right> | c | blink.cmp: Select Next | blink.cmp | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Left> | c | blink.cmp: Select Prev | blink.cmp | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-Space> | c | blink.cmp: Show | blink.cmp | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-P> | c | blink.cmp: Select Prev | blink.cmp | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-N> | c | blink.cmp: Select Next | blink.cmp | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <End> | c | blink.cmp: Hide | blink.cmp | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-E> | c | blink.cmp: Cancel | blink.cmp | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Tab> | c | blink.cmp: Show And Insert Or Accept Single, Select Next | blink.cmp | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-expand-repeat | i | LuaSnip: Repeat last node expansion | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-delete-check | i | LuaSnip: Removes current snippet from jumplist | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-jump-prev | i | LuaSnip: Jump to the previous node | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-jump-next | i | LuaSnip: Jump to the next node | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-prev-choice | i | LuaSnip: Change to the previous choice from the choiceNode | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-next-choice | i | LuaSnip: Change to the next choice from the choiceNode | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-expand-snippet | i | LuaSnip: Expand the current snippet | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-expand-or-jump | i | LuaSnip: Expand or jump in the current snippet | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <CR> | i | autopairs completion confirm | user | global | rust, pdf, typst, python, config, markdown, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| j | i | Lua callback /home/yaroslav/.local/share/nvim/lazy/better-escape.nvim/lua/better_escape.lua:110 | better-escape.nvim | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| k | i | Lua callback /home/yaroslav/.local/share/nvim/lazy/better-escape.nvim/lua/better_escape.lua:110 | better-escape.nvim | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-jump-prev | s | LuaSnip: Jump to the previous node | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-jump-next | s | LuaSnip: Jump to the next node | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-prev-choice | s | LuaSnip: Change to the previous choice from the choiceNode | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-next-choice | s | LuaSnip: Change to the next choice from the choiceNode | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-expand-snippet | s | LuaSnip: Expand the current snippet | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-expand-or-jump | s | LuaSnip: Expand or jump in the current snippet | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Plug>luasnip-expand-repeat | s | LuaSnip: Repeat last node expansion | LuaSnip | global | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space la | x | LSP code action | AstroNvim | buffer | rust, pdf, typst, python, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| [[ | x | :call rust#Jump('v', 'Back')<CR> | Neovim | buffer | rust, pdf, tui:rust | not mapped / verify grammar |
| ]] | x | :call rust#Jump('v', 'Forward')<CR> | Neovim | buffer | rust, pdf, tui:rust | not mapped / verify grammar |
| Space lA | n | LSP source action | AstroNvim | buffer | rust, pdf, typst, python, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space lL | n | LSP CodeLens run | AstroNvim | buffer | rust, pdf, typst, tui:rust, tui:typst | not mapped / verify grammar |
| Space uL | n | Toggle CodeLens | AstroNvim | buffer | rust, pdf, typst, tui:rust, tui:typst | not mapped / verify grammar |
| Space uY | n | Toggle LSP semantic highlight (buffer) | AstroNvim | buffer | rust, pdf, typst, python, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space lf | n | Format buffer | AstroNvim | buffer | rust, pdf, tui:rust | lsp.format |
| Space ll | n | LSP CodeLens refresh | AstroNvim | buffer | rust, pdf, typst, tui:rust, tui:typst | not mapped / verify grammar |
| Space uf | n | Toggle autoformatting (buffer) | AstroNvim | buffer | rust, pdf, typst, tui:rust, tui:typst | not mapped / verify grammar |
| Space uF | n | Toggle autoformatting (global) | AstroNvim | buffer | rust, pdf, typst, tui:rust, tui:typst | not mapped / verify grammar |
| Space lI | n | Null-ls information | Neovim | buffer | rust, pdf, typst, python, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space lG | n | Search workspace symbols | AstroNvim | buffer | rust, pdf, typst, python, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space u? | n | Toggle automatic signature help | AstroNvim | buffer | rust, pdf, typst, python, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space lh | n | Signature help | AstroNvim | buffer | rust, pdf, typst, python, tui:rust, tui:python, tui:typst | lsp.signature |
| Space uh | n | Toggle LSP inlay hints (buffer) | AstroNvim | buffer | rust, pdf, typst, python, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space lr | n | Rename current symbol | AstroNvim | buffer | rust, pdf, typst, python, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space uH | n | Toggle LSP inlay hints (global) | AstroNvim | buffer | rust, pdf, typst, python, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space lR | n | Search references | AstroNvim | buffer | rust, pdf, typst, python, tui:rust, tui:python, tui:typst | lsp.references |
| Space la | n | LSP code action | AstroNvim | buffer | rust, pdf, typst, python, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space li | n | LSP information | Neovim | buffer | rust, pdf, typst, python, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| [[ | n | :call rust#Jump('n', 'Back')<CR> | Neovim | buffer | rust, pdf, tui:rust | not mapped / verify grammar |
| ]] | n | :call rust#Jump('n', 'Forward')<CR> | Neovim | buffer | rust, pdf, tui:rust | not mapped / verify grammar |
| gK | n | Signature help | AstroNvim | buffer | rust, pdf, typst, python, tui:rust, tui:python, tui:typst | lsp.signature |
| gy | n | Definition of current type | AstroNvim | buffer | rust, pdf, python, tui:rust, tui:python | not mapped / verify grammar |
| gD | n | Declaration of current symbol | AstroNvim | buffer | rust, pdf, python, tui:rust, tui:python | not mapped / verify grammar |
| gd | n | Show the definition of current symbol | AstroNvim | buffer | rust, pdf, typst, python, tui:rust, tui:python, tui:typst | lsp.definition |
| [[ | o | :call rust#Jump('o', 'Back')<CR> | Neovim | buffer | rust, pdf, tui:rust | not mapped / verify grammar |
| ]] | o | :call rust#Jump('o', 'Forward')<CR> | Neovim | buffer | rust, pdf, tui:rust | not mapped / verify grammar |
| <Tab> | v | blink.cmp: Select Next, Snippet Forward, <Custom Fn> | blink.cmp | buffer | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space la | v | LSP code action | AstroNvim | buffer | rust, pdf, typst, python, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| [[ | v | :call rust#Jump('v', 'Back')<CR> | Neovim | buffer | rust, pdf, tui:rust | not mapped / verify grammar |
| ]] | v | :call rust#Jump('v', 'Forward')<CR> | Neovim | buffer | rust, pdf, tui:rust | not mapped / verify grammar |
| <S-Tab> | v | blink.cmp: Select Prev, Snippet Backward, <Custom Fn> | blink.cmp | buffer | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Down> | i | blink.cmp: Select Next | blink.cmp | buffer | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Up> | i | blink.cmp: Select Prev | blink.cmp | buffer | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-U> | i | blink.cmp: Scroll Documentation Up | blink.cmp | buffer | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-D> | i | blink.cmp: Scroll Documentation Down | blink.cmp | buffer | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-J> | i | blink.cmp: Select Next | blink.cmp | buffer | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-Y> | i | blink.cmp: Select And Accept | blink.cmp | buffer | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <S-Tab> | i | blink.cmp: Select Prev, Snippet Backward, <Custom Fn> | blink.cmp | buffer | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-F> | i | blink.cmp: Scroll Documentation Down | blink.cmp | buffer | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-Space> | i | blink.cmp: Show, Show Documentation, Hide Documentation | blink.cmp | buffer | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-B> | i | blink.cmp: Scroll Documentation Up | blink.cmp | buffer | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-P> | i | blink.cmp: Select Prev, Show | blink.cmp | buffer | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-N> | i | blink.cmp: Select Next, Show | blink.cmp | buffer | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-K> | i | blink.cmp: Select Prev | blink.cmp | buffer | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <C-E> | i | blink.cmp: Hide | blink.cmp | buffer | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <BS> | i | autopairs delete | nvim-autopairs | buffer | rust, pdf, typst, python, config, markdown, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <M-e> | i | autopairs fastwrap | user | buffer | rust, pdf, typst, python, config, markdown, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Tab> | i | blink.cmp: Select Next, Snippet Forward, <Custom Fn> | blink.cmp | buffer | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <CR> | i | blink.cmp: Accept | blink.cmp | buffer | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| " | i | autopairs map key | nvim-autopairs | buffer | rust, pdf, typst, python, config, markdown, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ' | i | autopairs map key | nvim-autopairs | buffer | rust, pdf, typst, python, config, markdown, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ( | i | autopairs map key | nvim-autopairs | buffer | rust, pdf, typst, python, config, markdown, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ) | i | autopairs map key | nvim-autopairs | buffer | rust, pdf, typst, python, config, markdown, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| > | i | Lua callback /home/yaroslav/.local/share/nvim/lazy/nvim-ts-autotag/lua/nvim-ts-autotag/internal.lua:473 | nvim-ts-autotag | buffer | rust, pdf, markdown, tui:markdown, tui:rust | not mapped / verify grammar |
| [ | i | autopairs map key | nvim-autopairs | buffer | rust, pdf, typst, python, config, markdown, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ] | i | autopairs map key | nvim-autopairs | buffer | rust, pdf, typst, python, config, markdown, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| ` | i | autopairs map key | nvim-autopairs | buffer | rust, pdf, typst, python, config, markdown, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| { | i | autopairs map key | nvim-autopairs | buffer | rust, pdf, typst, python, config, markdown, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| } | i | autopairs map key | nvim-autopairs | buffer | rust, pdf, typst, python, config, markdown, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <Tab> | s | blink.cmp: Select Next, Snippet Forward, <Custom Fn> | blink.cmp | buffer | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| <S-Tab> | s | blink.cmp: Select Prev, Snippet Backward, <Custom Fn> | blink.cmp | buffer | rust, pdf, typst, python, config, markdown, tui:pdf, tui:config, tui:markdown, tui:rust, tui:python, tui:typst | not mapped / verify grammar |
| Space ip | n | Paste clipboard image | user | global | typst, python, config, markdown, tui:typst | not mapped / verify grammar |
| <Plug>PlenaryTestFile | n | :lua require('plenary.test_harness').test_file(vim.fn.expand("%:p"))<CR> | plenary.nvim | global | typst, python, config, markdown, tui:typst | not mapped / verify grammar |
| <M-S> | n | Capture selected screen area | user | global | typst, python, config, markdown, tui:typst | not mapped / verify grammar |
| <M-S> | i | Capture selected screen area | user | global | typst, python, config, markdown, tui:typst | not mapped / verify grammar |
| Space ip | i | Paste clipboard image | user | global | typst, python, config, markdown, tui:typst | not mapped / verify grammar |
| Space lf | n | Format Typst | user | buffer | typst, tui:typst | lsp.format |
| [A | x | Previous argument end | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [K | x | Previous block end | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [F | x | Previous function end | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [k | x | Previous block start | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [a | x | Previous argument start | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [f | x | Previous function start | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [M | x | :call <SNR>46_Python_jump('x', '\v\S\n*(^(\s*\n*)*(class\|def\|async def)\|^\S)', 'Wb', v:count1, 0)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| [m | x | :call <SNR>46_Python_jump('x', '\v^\s*(class\|def\|async def)>', 'Wb', v:count1)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| [] | x | :call <SNR>46_Python_jump('x', '\v\S.*\n+(def\|class)', 'Wb', v:count1, 0)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| [[ | x | :call <SNR>46_Python_jump('x', '\v^(class\|def\|async def)>', 'Wb', v:count1)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| ]F | x | Next function end | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ]K | x | Next block end | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ]A | x | Next argument end | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ]a | x | Next argument start | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ]f | x | Next function start | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ]k | x | Next block start | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ]M | x | :call <SNR>46_Python_jump('x', '\v\S\n*(%$\|^(\s*\n*)*(class\|def\|async def)\|^\S)', 'W', v:count1, 0)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| ]m | x | :call <SNR>46_Python_jump('x', '\v%$\|^\s*(class\|def\|async def)>', 'W', v:count1)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| ][ | x | :call <SNR>46_Python_jump('x', '\v%$\|\S.*\n+(def\|class)', 'W', v:count1, 0)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| ]] | x | :call <SNR>46_Python_jump('x', '\v%$\|^(class\|def\|async def)>', 'W', v:count1)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| ak | x | around block | user | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| af | x | around function  | user | buffer | python, tui:python | not mapped / verify grammar |
| ao | x | around loop | user | buffer | python, tui:python | not mapped / verify grammar |
| ac | x | around class | user | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| aa | x | around argument | user | buffer | python, tui:python | not mapped / verify grammar |
| a? | x | around conditional | user | buffer | python, tui:python | not mapped / verify grammar |
| grm | x | Shrink selection to previous named node | nvim-treesitter | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| grc | x | Increment selection to surrounding scope | nvim-treesitter | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| grn | x | Increment selection to named node | nvim-treesitter | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| if | x | inside function  | user | buffer | python, tui:python | not mapped / verify grammar |
| ik | x | inside block | user | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| io | x | inside loop | user | buffer | python, tui:python | not mapped / verify grammar |
| ic | x | inside class | user | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ia | x | inside argument | user | buffer | python, tui:python | not mapped / verify grammar |
| i? | x | inside conditional | user | buffer | python, tui:python | not mapped / verify grammar |
| <lt>A | n | Swap previous argument | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| <lt>F | n | Swap previous function | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| <lt>K | n | Swap previous block | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| >A | n | Swap next argument | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| >F | n | Swap next function | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| >K | n | Swap next block | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [A | n | Previous argument end | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [K | n | Previous block end | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [F | n | Previous function end | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [k | n | Previous block start | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [a | n | Previous argument start | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [f | n | Previous function start | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [M | n | :<C-U>call <SNR>46_Python_jump('n', '\v\S\n*(^(\s*\n*)*(class\|def\|async def)\|^\S)', 'Wb', v:count1, 0)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| [m | n | :<C-U>call <SNR>46_Python_jump('n', '\v^\s*(class\|def\|async def)>', 'Wb', v:count1)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| [] | n | :<C-U>call <SNR>46_Python_jump('n', '\v\S.*\n+(def\|class)', 'Wb', v:count1, 0)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| [[ | n | :<C-U>call <SNR>46_Python_jump('n', '\v^(class\|def\|async def)>', 'Wb', v:count1)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| ]F | n | Next function end | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ]K | n | Next block end | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ]A | n | Next argument end | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ]a | n | Next argument start | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ]f | n | Next function start | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ]k | n | Next block start | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ]M | n | :<C-U>call <SNR>46_Python_jump('n', '\v\S\n*(%$\|^(\s*\n*)*(class\|def\|async def)\|^\S)', 'W', v:count1, 0)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| ]m | n | :<C-U>call <SNR>46_Python_jump('n', '\v%$\|^\s*(class\|def\|async def)>', 'W', v:count1)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| ][ | n | :<C-U>call <SNR>46_Python_jump('n', '\v%$\|\S.*\n+(def\|class)', 'W', v:count1, 0)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| ]] | n | :<C-U>call <SNR>46_Python_jump('n', '\v%$\|^(class\|def\|async def)>', 'W', v:count1)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| gnn | n | Start selecting nodes with nvim-treesitter | nvim-treesitter | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [A | o | Previous argument end | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [K | o | Previous block end | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [F | o | Previous function end | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [k | o | Previous block start | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [a | o | Previous argument start | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [f | o | Previous function start | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [M | o | :call <SNR>46_Python_jump('o', '\v\S\n*(^(\s*\n*)*(class\|def\|async def)\|^\S)', 'Wb', v:count1, 0)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| [m | o | :call <SNR>46_Python_jump('o', '\v^\s*(class\|def\|async def)>', 'Wb', v:count1)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| [] | o | :call <SNR>46_Python_jump('o', '\v\S.*\n+(def\|class)', 'Wb', v:count1, 0)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| [[ | o | :call <SNR>46_Python_jump('o', '\v^(class\|def\|async def)>', 'Wb', v:count1)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| ]F | o | Next function end | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ]K | o | Next block end | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ]A | o | Next argument end | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ]a | o | Next argument start | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ]f | o | Next function start | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ]k | o | Next block start | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ]M | o | :call <SNR>46_Python_jump('o', '\v\S\n*(%$\|^(\s*\n*)*(class\|def\|async def)\|^\S)', 'W', v:count1, 0)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| ]m | o | :call <SNR>46_Python_jump('o', '\v%$\|^\s*(class\|def\|async def)>', 'W', v:count1)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| ][ | o | :call <SNR>46_Python_jump('o', '\v%$\|\S.*\n+(def\|class)', 'W', v:count1, 0)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| ]] | o | :call <SNR>46_Python_jump('o', '\v%$\|^(class\|def\|async def)>', 'W', v:count1)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| ak | o | around block | user | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| af | o | around function  | user | buffer | python, tui:python | not mapped / verify grammar |
| ao | o | around loop | user | buffer | python, tui:python | not mapped / verify grammar |
| ac | o | around class | user | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| aa | o | around argument | user | buffer | python, tui:python | not mapped / verify grammar |
| a? | o | around conditional | user | buffer | python, tui:python | not mapped / verify grammar |
| if | o | inside function  | user | buffer | python, tui:python | not mapped / verify grammar |
| ik | o | inside block | user | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| io | o | inside loop | user | buffer | python, tui:python | not mapped / verify grammar |
| ic | o | inside class | user | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ia | o | inside argument | user | buffer | python, tui:python | not mapped / verify grammar |
| i? | o | inside conditional | user | buffer | python, tui:python | not mapped / verify grammar |
| [A | v | Previous argument end | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [K | v | Previous block end | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [F | v | Previous function end | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [k | v | Previous block start | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [a | v | Previous argument start | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [f | v | Previous function start | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| [M | v | :call <SNR>46_Python_jump('x', '\v\S\n*(^(\s*\n*)*(class\|def\|async def)\|^\S)', 'Wb', v:count1, 0)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| [m | v | :call <SNR>46_Python_jump('x', '\v^\s*(class\|def\|async def)>', 'Wb', v:count1)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| [] | v | :call <SNR>46_Python_jump('x', '\v\S.*\n+(def\|class)', 'Wb', v:count1, 0)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| [[ | v | :call <SNR>46_Python_jump('x', '\v^(class\|def\|async def)>', 'Wb', v:count1)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| ]F | v | Next function end | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ]K | v | Next block end | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ]A | v | Next argument end | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ]a | v | Next argument start | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ]f | v | Next function start | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ]k | v | Next block start | nvim-treesitter-textobjects | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ]M | v | :call <SNR>46_Python_jump('x', '\v\S\n*(%$\|^(\s*\n*)*(class\|def\|async def)\|^\S)', 'W', v:count1, 0)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| ]m | v | :call <SNR>46_Python_jump('x', '\v%$\|^\s*(class\|def\|async def)>', 'W', v:count1)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| ][ | v | :call <SNR>46_Python_jump('x', '\v%$\|\S.*\n+(def\|class)', 'W', v:count1, 0)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| ]] | v | :call <SNR>46_Python_jump('x', '\v%$\|^(class\|def\|async def)>', 'W', v:count1)<CR> | Neovim | buffer | python | not mapped / verify grammar |
| ak | v | around block | user | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| af | v | around function  | user | buffer | python, tui:python | not mapped / verify grammar |
| ao | v | around loop | user | buffer | python, tui:python | not mapped / verify grammar |
| ac | v | around class | user | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| aa | v | around argument | user | buffer | python, tui:python | not mapped / verify grammar |
| a? | v | around conditional | user | buffer | python, tui:python | not mapped / verify grammar |
| grm | v | Shrink selection to previous named node | nvim-treesitter | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| grc | v | Increment selection to surrounding scope | nvim-treesitter | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| grn | v | Increment selection to named node | nvim-treesitter | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| if | v | inside function  | user | buffer | python, tui:python | not mapped / verify grammar |
| ik | v | inside block | user | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| io | v | inside loop | user | buffer | python, tui:python | not mapped / verify grammar |
| ic | v | inside class | user | buffer | python, markdown, tui:markdown, tui:python | not mapped / verify grammar |
| ia | v | inside argument | user | buffer | python, tui:python | not mapped / verify grammar |
| i? | v | inside conditional | user | buffer | python, tui:python | not mapped / verify grammar |
| [[ | x | :<C-U>exe "normal! gv"\|call search('\%(^#\{1,5\}\s\+\S\\|^\S.*\n^[=-]\+$\)', "bsW")<CR> | Neovim | buffer | markdown, tui:markdown | not mapped / verify grammar |
| ]] | x | :<C-U>exe "normal! gv"\|call search('\%(^#\{1,5\}\s\+\S\\|^\S.*\n^[=-]\+$\)', "sW")<CR> | Neovim | buffer | markdown, tui:markdown | not mapped / verify grammar |
| [[ | n | Jump to previous section | Neovim | buffer | markdown, tui:markdown | not mapped / verify grammar |
| ]] | n | Jump to next section | Neovim | buffer | markdown, tui:markdown | not mapped / verify grammar |
| gO | n | Show an Outline of the current buffer | Neovim | buffer | markdown, tui:markdown | not mapped / verify grammar |
| [[ | v | :<C-U>exe "normal! gv"\|call search('\%(^#\{1,5\}\s\+\S\\|^\S.*\n^[=-]\+$\)', "bsW")<CR> | Neovim | buffer | markdown, tui:markdown | not mapped / verify grammar |
| ]] | v | :<C-U>exe "normal! gv"\|call search('\%(^#\{1,5\}\s\+\S\\|^\S.*\n^[=-]\+$\)', "sW")<CR> | Neovim | buffer | markdown, tui:markdown | not mapped / verify grammar |
| - | i | autopairs map key | nvim-autopairs | buffer | markdown, tui:markdown | not mapped / verify grammar |
| [M | v | :call <SNR>54_Python_jump('x', '\v\S\n*(^(\s*\n*)*(class\|def\|async def)\|^\S)', 'Wb', v:count1, 0)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| [m | v | :call <SNR>54_Python_jump('x', '\v^\s*(class\|def\|async def)>', 'Wb', v:count1)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| [] | v | :call <SNR>54_Python_jump('x', '\v\S.*\n+(def\|class)', 'Wb', v:count1, 0)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| [[ | v | :call <SNR>54_Python_jump('x', '\v^(class\|def\|async def)>', 'Wb', v:count1)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| ]M | v | :call <SNR>54_Python_jump('x', '\v\S\n*(%$\|^(\s*\n*)*(class\|def\|async def)\|^\S)', 'W', v:count1, 0)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| ]m | v | :call <SNR>54_Python_jump('x', '\v%$\|^\s*(class\|def\|async def)>', 'W', v:count1)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| ][ | v | :call <SNR>54_Python_jump('x', '\v%$\|\S.*\n+(def\|class)', 'W', v:count1, 0)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| ]] | v | :call <SNR>54_Python_jump('x', '\v%$\|^(class\|def\|async def)>', 'W', v:count1)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| [M | o | :call <SNR>54_Python_jump('o', '\v\S\n*(^(\s*\n*)*(class\|def\|async def)\|^\S)', 'Wb', v:count1, 0)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| [m | o | :call <SNR>54_Python_jump('o', '\v^\s*(class\|def\|async def)>', 'Wb', v:count1)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| [] | o | :call <SNR>54_Python_jump('o', '\v\S.*\n+(def\|class)', 'Wb', v:count1, 0)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| [[ | o | :call <SNR>54_Python_jump('o', '\v^(class\|def\|async def)>', 'Wb', v:count1)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| ]M | o | :call <SNR>54_Python_jump('o', '\v\S\n*(%$\|^(\s*\n*)*(class\|def\|async def)\|^\S)', 'W', v:count1, 0)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| ]m | o | :call <SNR>54_Python_jump('o', '\v%$\|^\s*(class\|def\|async def)>', 'W', v:count1)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| ][ | o | :call <SNR>54_Python_jump('o', '\v%$\|\S.*\n+(def\|class)', 'W', v:count1, 0)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| ]] | o | :call <SNR>54_Python_jump('o', '\v%$\|^(class\|def\|async def)>', 'W', v:count1)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| [M | x | :call <SNR>54_Python_jump('x', '\v\S\n*(^(\s*\n*)*(class\|def\|async def)\|^\S)', 'Wb', v:count1, 0)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| [m | x | :call <SNR>54_Python_jump('x', '\v^\s*(class\|def\|async def)>', 'Wb', v:count1)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| [] | x | :call <SNR>54_Python_jump('x', '\v\S.*\n+(def\|class)', 'Wb', v:count1, 0)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| [[ | x | :call <SNR>54_Python_jump('x', '\v^(class\|def\|async def)>', 'Wb', v:count1)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| ]M | x | :call <SNR>54_Python_jump('x', '\v\S\n*(%$\|^(\s*\n*)*(class\|def\|async def)\|^\S)', 'W', v:count1, 0)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| ]m | x | :call <SNR>54_Python_jump('x', '\v%$\|^\s*(class\|def\|async def)>', 'W', v:count1)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| ][ | x | :call <SNR>54_Python_jump('x', '\v%$\|\S.*\n+(def\|class)', 'W', v:count1, 0)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| ]] | x | :call <SNR>54_Python_jump('x', '\v%$\|^(class\|def\|async def)>', 'W', v:count1)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| [M | n | :<C-U>call <SNR>54_Python_jump('n', '\v\S\n*(^(\s*\n*)*(class\|def\|async def)\|^\S)', 'Wb', v:count1, 0)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| [m | n | :<C-U>call <SNR>54_Python_jump('n', '\v^\s*(class\|def\|async def)>', 'Wb', v:count1)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| [] | n | :<C-U>call <SNR>54_Python_jump('n', '\v\S.*\n+(def\|class)', 'Wb', v:count1, 0)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| [[ | n | :<C-U>call <SNR>54_Python_jump('n', '\v^(class\|def\|async def)>', 'Wb', v:count1)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| ]M | n | :<C-U>call <SNR>54_Python_jump('n', '\v\S\n*(%$\|^(\s*\n*)*(class\|def\|async def)\|^\S)', 'W', v:count1, 0)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| ]m | n | :<C-U>call <SNR>54_Python_jump('n', '\v%$\|^\s*(class\|def\|async def)>', 'W', v:count1)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| ][ | n | :<C-U>call <SNR>54_Python_jump('n', '\v%$\|\S.*\n+(def\|class)', 'W', v:count1, 0)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| ]] | n | :<C-U>call <SNR>54_Python_jump('n', '\v%$\|^(class\|def\|async def)>', 'W', v:count1)<CR> | Neovim | buffer | tui:python | not mapped / verify grammar |
| h | grammar | h | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| j | grammar | j | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| k | grammar | k | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| l | grammar | l | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| w | grammar | w | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| W | grammar | W | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| b | grammar | b | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| B | grammar | B | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| e | grammar | e | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| E | grammar | E | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| ge | grammar | ge | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| gE | grammar | gE | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| 0 | grammar | 0 | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| ^ | grammar | ^ | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| $ | grammar | $ | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| gg | grammar | gg | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| G | grammar | G | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| H | grammar | H | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| M | grammar | M | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| L | grammar | L | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| f{char} | grammar | f{char} | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| F{char} | grammar | F{char} | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| t{char} | grammar | t{char} | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| T{char} | grammar | T{char} | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| ; | grammar | ; | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| , | grammar | , | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| % | grammar | % | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| { | grammar | { | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| } | grammar | } | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| ( | grammar | ( | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| ) | grammar | ) | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| d{motion} | grammar | d{motion} | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| c{motion} | grammar | c{motion} | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| y{motion} | grammar | y{motion} | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| >{motion} | grammar | >{motion} | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| <{motion} | grammar | <{motion} | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| ={motion} | grammar | ={motion} | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| g~{motion} | grammar | g~{motion} | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| gu{motion} | grammar | gu{motion} | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| gU{motion} | grammar | gU{motion} | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| iw | grammar | iw | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| aw | grammar | aw | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| i" | grammar | i" | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| a" | grammar | a" | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| i' | grammar | i' | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| a' | grammar | a' | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| i( | grammar | i( | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| a( | grammar | a( | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| i[ | grammar | i[ | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| a[ | grammar | a[ | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| i{ | grammar | i{ | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| a{ | grammar | a{ | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| ip | grammar | ip | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| ap | grammar | ap | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| i | grammar | i | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| a | grammar | a | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| I | grammar | I | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| A | grammar | A | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| o | grammar | o | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| O | grammar | O | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| Esc | grammar | Esc | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| v | grammar | v | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| V | grammar | V | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| Ctrl+v | grammar | Ctrl+v | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| : | grammar | : | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| / | grammar | / | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| ? | grammar | ? | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| u | grammar | u | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| Ctrl+r | grammar | Ctrl+r | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| p | grammar | p | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| P | grammar | P | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
| . | grammar | . | Vim builtin | builtin | Vim modes as documented | not mapped / verify grammar |
