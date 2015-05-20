set -e
set -o pipefail

export TS=`date +%s`
echo TS is $TS

# pushd ../whisper && python ../whisper/setup.py install > /dev/null && popd

# export FNAME="1-5-5-5-10-5"
# export ARCHIVE="1:5 5:5 10:5"

# export FNAME="1s-6h-1h-1d-24h-20y"
# export ARCHIVE="1s:6h 1h:1d 24h:20y"

export FNAME="1s-24h-1h-30d-24h-30y"
export ARCHIVE="1s:24h 1h:30d 24h:30y"

rm test/fixtures/$FNAME.wsp.{python,rust}
whisper-create.py test/fixtures/$FNAME.wsp.python $ARCHIVE
whisper-create.py test/fixtures/$FNAME.wsp.rust $ARCHIVE

echo "PYTHON TIME"
time {
	for i in `seq 1 100000`;
	do
		whisper-update.py test/fixtures/$FNAME.wsp.python $TS:10
	done
}

echo "RUST TIME"
time {
	for i in `seq 1 100000`;
	do
		RUST_LOG=warning target/release/whisper update test/fixtures/$FNAME.wsp.rust $TS 10
	done
}

diff test/fixtures/$FNAME.wsp.*

if [[ $? -eq 0 ]]; then
	echo "whisper files match"
fi
