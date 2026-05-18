# Source this file after adding pi-observe to PATH.
# These explicit aliases avoid replacing original tools unexpectedly.

alias claude-observed='pi-observe run --tool claude-code --role coding-agent -- claude'
# Editor launchers are nonbillable context by default; use manual markers or observed tasks for billable work.
alias cursor-observed='pi-observe run --tool cursor --role editor-session --nonbillable -- cursor .'
alias code-observed='pi-observe run --tool vscode --role editor-session --nonbillable -- code .'
alias antigravity-observed='pi-observe run --tool google-antigravity --role coding-agent --nonbillable -- antigravity .'

pi-arch() { pi-observe run --tool pi-team --role delegate-architect -- "$@"; }
pi-dev()  { pi-observe run --tool pi-team --role delegate-developer -- "$@"; }
pi-rev()  { pi-observe run --tool pi-team --role delegate-reviewer -- "$@"; }
pi-copilot-suggest() { pi-observe run --tool copilot-cli --role suggest -- gh copilot suggest "$@"; }
pi-copilot-explain() { pi-observe run --tool copilot-cli --role explain -- gh copilot explain "$@"; }
