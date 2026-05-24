# Shell Completions

`eksup` supports shell completion for **bash**, **zsh**, **fish**, **powershell**, and **elvish**.

## From the `eksup` binary

If you have `eksup` installed (via `cargo install`, Homebrew, or any other method), generate the completion script with the `completion` subcommand and write it to your shell's completion directory.

### Bash

```sh
eksup completion bash > /etc/bash_completion.d/eksup
# or, for current user:
eksup completion bash > ~/.local/share/bash-completion/completions/eksup
```

### Zsh

```sh
mkdir -p ~/.zfunc
eksup completion zsh > ~/.zfunc/_eksup
```

Then ensure `~/.zfunc` is on your `fpath` in `~/.zshrc`:

```sh
fpath=(~/.zfunc $fpath)
autoload -U compinit && compinit
```

### Fish

```sh
eksup completion fish > ~/.config/fish/completions/eksup.fish
```

### PowerShell

```powershell
eksup completion powershell | Out-String | Invoke-Expression
```

Or to persist across sessions, add the above to your `$PROFILE`.

### Elvish

```sh
eksup completion elvish > ~/.config/elvish/lib/eksup.elv
```

Then `use eksup` in your `rc.elv`.

## From a release tarball

Release tarballs at <https://github.com/clowdhaus/eksup/releases> include pre-generated completion files under `completions/` and a man page at `man/eksup.1`. Most package managers (Homebrew, AUR, apt, etc.) install these to the appropriate system locations automatically.

If installing manually:

```sh
# Bash (system-wide)
sudo install -Dm644 completions/eksup.bash /etc/bash_completion.d/eksup

# Zsh (per-user)
install -Dm644 completions/_eksup ~/.zfunc/_eksup

# Fish (per-user)
install -Dm644 completions/eksup.fish ~/.config/fish/completions/eksup.fish

# Man page
sudo install -Dm644 man/eksup.1 /usr/local/share/man/man1/eksup.1
```

## Man page

The same `eksup` binary can render its man page on demand:

```sh
eksup man > /usr/local/share/man/man1/eksup.1
sudo mandb     # refresh man's index (Linux)
man eksup
```
