# popdo

A tiny todo list for tmux popups. Press `Ctrl-T` in tmux to see your todos, then press it again to close the popup.

Each git project gets its own list, and there is one global list for everything else.

```
╭ popdo │ global ──────────────────── 1/2 done ╮
│ ○ fix the CI                                 │
│▌✓ write the README                           │
╰──────────────────────────────────────────────╯
 a add  e edit  space done  d delete  D clear done
 j/k move  J/K reorder  tab switch list  q close
```

Colors follow the Tokyo Night theme.

## Install

```bash
cargo install --path .
```

Add these lines to `~/.tmux.conf`, then reload tmux:

```tmux
# Ctrl-T opens popdo in the current pane's folder
bind -n C-t display-popup -E -B -w 60% -h 50% -d "#{pane_current_path}" popdo

# Optional: number of open todos of the current project, in the status bar
set -ga status-right "#(popdo count --dir '#{pane_current_path}')"
```

## Usage

```bash
popdo                    # open the list
popdo add buy milk       # add to the current project (quotes are optional)
popdo add -g buy milk    # add to the global list
popdo count              # print how many todos are not done yet
```

All commands accept `--dir <folder>` to use another folder than the current one.

In the list:

| Key | Action |
|---|---|
| `a` | add a todo |
| `e` | edit the selected todo |
| `space` | mark as done / not done |
| `d`, then `y` | delete the selected todo |
| `D`, then `y` | delete all done todos |
| `j` / `k` or arrows | move |
| `J` / `K` | move the selected todo down / up |
| `tab` | switch between the project list and the global list |
| `q`, `Esc` or `Ctrl-T` | close |

When typing, `Enter` saves and `Esc` cancels. Every change is saved right away.

## Projects

popdo looks for a `.git` folder in the current folder and its parents. The
folder that contains it is the project. A git worktree uses the list of its
main repository, so all worktrees of a repository share one list. Outside a
git project, popdo shows the global list.

## Storage

Todos are saved as JSON in your local data folder
(`~/Library/Application Support/popdo/todos.json` on macOS).
Set `POPDO_FILE` to use another file. Files from popdo 0.1 (a plain list)
still load: their todos become the global list.

## Development

```bash
cargo test
cargo clippy --all-targets
```
