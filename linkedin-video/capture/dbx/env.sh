# source me
W=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/vid/work-dbx
B=$W/dbx-bin
T="tmux -L dbxvid -f $W/tmux.conf"
COLS=${COLS:-120}; ROWS=${ROWS:-40}
start() {
  $T kill-server 2>/dev/null; sleep 0.3
  mkdir -p $W/run/bin $W/run/home; ln -sf $B $W/run/bin/dbx; cp $W/shop.db $W/run/shop.db; cp $W/home/dbx.toml $W/run/config.toml; chmod 600 $W/run/config.toml
  $T new-session -d -s v -x $COLS -y $ROWS -c $W/run \
    "env -i PATH=$W/run/bin:/usr/bin:/bin HOME=$W/run/home XDG_CONFIG_HOME=$W/run/home/.config TERM=xterm-256color COLORTERM=truecolor LANG=C.UTF-8 DBX_CONFIG=config.toml dbx; sleep 1000"
}
cap() { mkdir -p $W/caps; $T capture-pane -e -p -t v > $W/caps/$1.ansi; }
keys() { $T send-keys -t v "$@"; }
show() { $T capture-pane -p -t v; }
