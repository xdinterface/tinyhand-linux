printf 'sound = false\ntray = false\n' > $TINYHAND_CONFIG
th make-hand $E/opaque.png; echo "opaque=$?"
rm -rf $XDG_DATA_HOME; th make-hand $E/coral-hand.png "Coral test"; echo "good=$?"
TH_TIMEOUT=40 th settings > $E/settings.log 2>&1 &
sleep 2.5
cur 120 111; down; up; sleep 0.8; shot set-pointers
cur 120 138; down; up; sleep 0.8; shot set-appearance
cur 862 231; down; up; sleep 0.3
cur 120 165; down; up; sleep 0.8; shot set-drawing
cur 120 192; down; up; sleep 0.8
cur 380 311; down; up; sleep 0.4
cur 120 219; down; up; sleep 0.8; shot set-about
echo ---config; cat $TINYHAND_CONFIG
