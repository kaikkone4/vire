# Source this file after adding pi-observe to PATH.
# These explicit aliases avoid replacing original tools unexpectedly.

alias claude-observed='pi-observe run --tool claude-code --role coding-agent -- claude'
alias cursor-observed='pi-observe run --tool cursor --role editor-session -- cursor .'
alias code-observed='pi-observe run --tool vscode --role editor-session -- code .'
alias antigravity-observed='pi-observe run --tool google-antigravity --role coding-agent -- antigravity .'

pi-arch() { pi-observe run --tool pi-team --role delegate-architect -- "$@"; }
pi-dev()  { pi-observe run --tool pi-team --role delegate-developer -- "$@"; }
pi-rev()  { pi-observe run --tool pi-team --role delegate-reviewer -- "$@"; }
pi-copilot-suggest() { pi-observe run --tool copilot-cli --role suggest -- gh copilot suggest "$@"; }
pi-copilot-explain() { pi-observe run --tool copilot-cli --role explain -- gh copilot explain "$@"; }
