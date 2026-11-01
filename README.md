# popdo

A tiny todo list for tmux popups. Press `Ctrl-T` in tmux to see your todos, then press it again to close the popup.

```
┌ Todo (2) ─────────────────────────────────┐
│• Préparer la réunion de lundi             │
│• Fix the accent bug in protodo            │
│                                           │
└ q: close ─────────────────────────────────┘
```

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
popdo add buy milk   # add a todo (quotes are optional)
popdo                # show the list; q, Esc or Ctrl-T closes it
```

Todos are saved as JSON in your local data folder
(`~/Library/Application Support/popdo/todos.json` on macOS).
Set `POPDO_FILE` to use another file.

## Development

```bash
cargo test
cargo clippy --all-targets
```
