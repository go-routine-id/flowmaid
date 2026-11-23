# source me (take 2). SOCK/RUN/COLS/ROWS may be preset.
W=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/vid/work-dbx
W2=$W/take2
B=$W/dbx-bin
SOCK=${SOCK:-dbxvid2}
RUN=${RUN:-$W2/run}
T="tmux -L $SOCK -f $W/tmux.conf"
COLS=${COLS:-100}; ROWS=${ROWS:-30}
start() {
  $T kill-server 2>/dev/null; sleep 0.3
  rm -rf $RUN; mkdir -p $RUN/bin $RUN/home; ln -sf $B $RUN/bin/dbx; cp $W/shop.db $RUN/shop.db; cp $W/home/dbx.toml $RUN/config.toml; chmod 600 $RUN/config.toml
  $T new-session -d -s v -x $COLS -y $ROWS -c $RUN \
    "env -i PATH=$RUN/bin:/usr/bin:/bin HOME=$RUN/home XDG_CONFIG_HOME=$RUN/home/.config TERM=xterm-256color COLORTERM=truecolor LANG=C.UTF-8 DBX_CONFIG=config.toml dbx; sleep 1000"
}
capa() { mkdir -p $(dirname $1); $T capture-pane -e -p -t v > $1.ansi; $T capture-pane -p -t v > $1.txt; }
keys() { $T send-keys -t v "$@"; }
show() { $T capture-pane -p -t v; }
w() { python3 -c "import time;time.sleep($1)"; }
stop() { $T kill-server 2>/dev/null; }
