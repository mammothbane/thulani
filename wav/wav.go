package wav

// #define DR_WAV_IMPLEMENTATION
// #include "dr_wav.h"
import "C"
import (
	"fmt"

	"sync"

	"github.com/op/go-logging"
	"layeh.com/gopus"
)

// number of individual samples per channel per batch
const samplesPerChannelPerBatch = 1920

var log = logging.MustGetLogger("wav")

type Wav struct {
	filename *C.char
	enc      *gopus.Encoder
	wav      *C.drwav

	once sync.Once
	Done <-chan struct{}
	done chan<- struct{}
}

func New(filename string) (*Wav, error) {
	cfname := C.CString(filename)
	wav := C.drwav_open_file(cfname)
	if wav == nil {
		C.drwav_close(wav)
		return nil, fmt.Errorf("Unable to initialize drwav.")
	}

	if int(wav.channels) != 2 {
		C.drwav_close(wav)
		return nil, fmt.Errorf("Wrong number of channels!")
	}

	if int(wav.sampleRate) != 48000 {
		C.drwav_close(wav)
		return nil, fmt.Errorf("Wrong sample rate.")
	}

	enc, err := gopus.NewEncoder(int(wav.sampleRate), int(wav.channels), gopus.Audio)
	if err != nil {
		C.drwav_close(wav)
		return nil, err
	}

	done := make(chan struct{}, 1)
	return &Wav{
		filename: C.CString(filename),
		enc:      enc,
		wav:      wav,

		done: done,
		Done: done,
	}, nil
}

func (w *Wav) Stop() {
	w.once.Do(func() {
		close(w.done)
	})
}

func (w *Wav) Start(ch chan<- []byte) {
	go func() {
		defer w.Stop()

		samplesPerBatch := samplesPerChannelPerBatch * int(w.wav.channels)
		batchSize := samplesPerBatch * int(w.wav.bytesPerSample)

		buf := C.malloc(C.size_t(batchSize))
		defer C.free(buf)
		defer C.drwav_close(w.wav)

		elems := make([]int16, samplesPerBatch)
		idx := 0

		for i := 0; i*samplesPerBatch <= int(w.wav.totalSampleCount); i += 1 {
			readSamples := C.drwav_read_s16(w.wav, C.dr_uint64(samplesPerBatch), (*C.dr_int16)(buf))
			slc := (*[1 << 30]int16)(buf)[:readSamples:readSamples]

			readIdx := 0

			for {
				batchSamplesToFill := samplesPerBatch - idx
				readSamplesRemaining := int(readSamples) - readIdx

				// break if we don't have enough samples to fill the rest of the buffer
				if readSamplesRemaining < batchSamplesToFill {
					break
				}

				copy(elems[idx:], slc[readIdx:readIdx+batchSamplesToFill])
				idx = 0
				readIdx += batchSamplesToFill

				b, err := processPCM(w.wav, w.enc, elems[:])
				if err != nil {
					log.Errorf("error encoding pcm: %q", err)
					continue
				}

				select {
				case <-w.Done:
					return
				default:
				}
				select {
				case <-w.Done:
					return
				case ch <- b:
				}
			}

			batchSamplesToFill := samplesPerBatch - idx
			readSamplesRemaining := int(readSamples) - readIdx
			if readSamplesRemaining >= batchSamplesToFill {
				log.Fatalf("Had enough samples to fill batch after for loop.")
			}

			copy(elems[idx:], slc)
			idx += len(slc)

			if int(readSamples) < samplesPerBatch {
				break
			}
		}
	}()
}

func processPCM(wav *C.drwav, enc *gopus.Encoder, data []int16) ([]byte, error) {
	return enc.Encode(data, len(data)/int(wav.channels), len(data)*2)
}
