package wav

// #define DR_WAV_IMPLEMENTATION
// #include "dr_wav.h"
import "C"
import (
	"fmt"
)

const batchSize = 64

func Load(filename string) (<-chan [2]int16, error) {
	cfname := C.CString(filename)
	wav := C.drwav_open_file(cfname)
	if wav == nil {
		return nil, fmt.Errorf("Unable to initialize drwav.")
	}

	if int(wav.channels) != 2 {
		C.drwav_close(wav)
		return nil, fmt.Errorf("Wrong number of channels!")
	}

	if int(wav.sampleRate) != 44100 {
		C.drwav_close(wav)
		return nil, fmt.Errorf("Wrong sample rate.")
	}

	ch := make(chan [2]int16, 1024*32)

	go func() {
		buf := C.malloc(C.size_t(batchSize * wav.bytesPerSample))
		defer C.free(buf)
		defer C.drwav_close(wav)

		for i := 0; i < int(wav.totalSampleCount)/batchSize; i++ {
			readSamples := C.drwav_read_s16(wav, batchSize, (*C.dr_int16)(buf))

			slc := (*[1 << 30]int16)(buf)[:readSamples:readSamples]

			for i := 0; i < int(readSamples); i += 2 {
				ch <- [2]int16{slc[i], slc[i+1]}
			}

			if readSamples < batchSize {
				break
			}
		}
		close(ch)
	}()

	return ch, nil
}
