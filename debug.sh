export TS=`date +%s`
echo TS is $TS

# rm test/fixtures/1-5-5-5-10-5.wsp.{python,rust}

# whisper-create.py test/fixtures/1-5-5-5-10-5.wsp.python 1:5 5:5 10:5
# whisper-create.py test/fixtures/1-5-5-5-10-5.wsp.rust 1:5 5:5 10:5

whisper-update.py test/fixtures/1-5-5-5-10-5.wsp.python $TS:10
RUST_LOG=debug target/debug/whisper update test/fixtures/1-5-5-5-10-5.wsp.rust $TS 10
