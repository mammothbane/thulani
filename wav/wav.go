package wav

// #define DR_WAV_IMPLEMENTATION
// #include "dr_wav.h"
import "C"
import (
	"fmt"

	"github.com/op/go-logging"
	"layeh.com/gopus"
)

// number of individual samples per batch (counting all channels)
const samplesPerBatch = 1920

var log = logging.MustGetLogger("wav")

func Load(filename string) (<-chan []byte, <-chan struct{}, error) {
	cfname := C.CString(filename)
	wav := C.drwav_open_file(cfname)
	if wav == nil {
		return nil, nil, fmt.Errorf("Unable to initialize drwav.")
	}

	if int(wav.channels) != 2 {
		C.drwav_close(wav)
		return nil, nil, fmt.Errorf("Wrong number of channels!")
	}

	if int(wav.sampleRate) != 48000 {
		C.drwav_close(wav)
		return nil, nil, fmt.Errorf("Wrong sample rate.")
	}

	ch := make(chan []byte, 1024*32)
	enc, err := gopus.NewEncoder(int(wav.sampleRate), int(wav.channels), gopus.Audio)
	if err != nil {
		return nil, nil, err
	}

	doneCh := make(chan struct{})
	encoderCh := make(chan []int16, 2*48000*2)
	go func() {
		buf := C.malloc(C.size_t(samplesPerBatch * wav.bytesPerSample))
		defer C.free(buf)
		defer C.drwav_close(wav)

		for i := 0; i < (int(wav.totalSampleCount)/samplesPerBatch)+1; i++ {
			readSamples := C.drwav_read_s16(wav, samplesPerBatch, (*C.dr_int16)(buf))
			slc := (*[1 << 30]int16)(buf)[:readSamples:readSamples]

			encoderCh <- slc

			if readSamples < samplesPerBatch {
				break
			}
		}
		close(encoderCh)
	}()

	go func(channels int) {
		elems := []int16{}
		for v := range encoderCh {
			elems = append(elems, v...)

			if len(elems) > samplesPerBatch*channels {
				opus, err := enc.Encode(elems[:samplesPerBatch*channels], samplesPerBatch, samplesPerBatch*channels*2)
				elems = elems[samplesPerBatch*channels:]
				if err != nil {
					log.Errorf("Error encoding opus audio: %q", err)
					continue
				}
				ch <- opus
			}
		}

		close(ch)
		close(doneCh)
	}(int(wav.channels))

	return ch, doneCh, nil
}
