rm -rf $XDG_DATA_HOME
printf 'sound = false\ntray = false\n' > $TINYHAND_CONFIG
ID=$(th make-hand $E/coral-hand.png "Doomed" | awk '/^added/{print $2}')
TH_TIMEOUT=60 th > $E/th.log 2>&1 &
sleep 1
th pick $ID; th on; sleep 0.4; cur 600 400; sleep 0.4
shot del-before
rm $XDG_DATA_HOME/tinyhand/hands/$ID.png; printf '' > $XDG_DATA_HOME/tinyhand/hands/hands.toml
echo "# touch" >> $TINYHAND_CONFIG; sleep 0.5
cur 610 410; sleep 0.4
shot del-after
th quit
