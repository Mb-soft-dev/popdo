# popdo

A tiny todo list for tmux popups. Press `Ctrl-T` in tmux to see your todos, then press it again to close the popup.

```
╭ Todo ──────────────────────────── 1/3 done ╮
│ ○ Préparer la réunion de lundi             │
│▌✓ Buy bread                                │
│ ○ Écrire le README avec des exemples       │
╰────────────────────────────────────────────╯
 a add  e edit  space done  d delete  j/k move  q close
```

Colors follow the Tokyo Night theme.

## Install

```bash
cargo install --path .
```

Add this line to `~/.tmux.conf`, then reload tmux:

```tmux
bind -n C-t display-popup -E -B -w 60% -h 50% popdo
```

## Usage

```bash
popdo                # open the list
popdo add buy milk   # add a todo from the shell (quotes are optional)
```

In the list:

| Key | Action |
|---|---|
| `a` | add a todo |
| `e` | edit the selected todo |
| `space` | mark as done / not done |
| `d`, then `y` | delete the selected todo |
| `j` / `k` or arrows | move |
| `q`, `Esc` or `Ctrl-T` | close |

When typing, `Enter` saves and `Esc` cancels. Every change is saved right away.

Todos are saved as JSON in your local data folder
(`~/Library/Application Support/popdo/todos.json` on macOS).
Set `POPDO_FILE` to use another file.

## Development

```bash
cargo test
cargo clippy --all-targets
```
