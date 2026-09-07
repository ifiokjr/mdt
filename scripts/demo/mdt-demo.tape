# mdt terminal demo — write it once, sync it everywhere
#
# Records assets/mdt-demo.gif for the README (issue #126).
# Run from the repository root: vhs scripts/demo/mdt-demo.tape
# Requirements: cargo (to build the mdt binary) — everything else is created here.

Output assets/mdt-demo.gif

Set Shell bash
Set FontSize 22
Set Width 1120
Set Height 680
Set Margin 0
Set Padding 28
Set CursorBlink false
Set TypingSpeed 40ms
Set Theme {
    "name": "mdt",
    "background": "#0E1218",
    "foreground": "#E6EDF3",
    "cursor": "#14B8A6",
    "selection": "#6366F1",
    "black": "#0E1218",
    "red": "#F97583",
    "green": "#14B8A6",
    "yellow": "#E3B341",
    "blue": "#6366F1",
    "magenta": "#BC8CFF",
    "cyan": "#76E3EA",
    "white": "#E6EDF3",
    "brightBlack": "#5A6472",
    "brightRed": "#FF707E",
    "brightGreen": "#3DDBB4",
    "brightYellow": "#FFCB6B",
    "brightBlue": "#8A93FF",
    "brightMagenta": "#D2A8FF",
    "brightCyan": "#9CEDF0",
    "brightWhite": "#FFFFFF"
}

Hide
Type "cargo build -q -p mdt_cli 2>/dev/null" Enter
Enter
Type 'export PATH=$PWD/target/debug:$PATH' Enter
Type "rm -rf /tmp/mdt-demo && mkdir -p /tmp/mdt-demo && cd /tmp/mdt-demo" Enter
Type "printf '<!-- {@install} -->\n\nnpm install my-lib\n\n<!-- {/install} -->\n' > install.t.md" Enter
Type "printf '# my-lib\n\nA tiny example library.\n\n<!-- {=install} -->\n\nnpm install my-old-lib\n\n<!-- {/install} -->\n' > readme.md" Enter
Enter
Show

# The single source of truth
Type "cat install.t.md" Enter
Sleep 1.4s
Ctrl+L
Sleep 200ms

# The consumer has drifted
Type "cat readme.md" Enter
Sleep 1.4s
Ctrl+L
Sleep 200ms

# CI catches the drift
Type "mdt check" Enter
Sleep 1.6s
Ctrl+L
Sleep 200ms

# One command syncs it
Type "mdt update" Enter
Sleep 1s
Type "cat readme.md" Enter
Sleep 1.6s
Ctrl+L
Sleep 200ms

# Back to green
Type "mdt check" Enter
Sleep 1.8s