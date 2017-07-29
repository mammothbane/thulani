package downloader

import (
	"io/ioutil"
	"os"
	"os/exec"
	"strconv"
	"sync"
	"time"

	"github.com/mammothbane/thulani-go/wav"
)

// Downloader handles a download for a particular song.
type Downloader struct {
	Url string

	Start    time.Duration
	Duration time.Duration
	End      time.Duration

	pause chan wav.State
	once  sync.Once
	done  chan struct{}
	pb    chan *wavBundle

	info videoInfo
}

const clipTime = 10 * time.Second
const preloadCount = 5

func NewDownload(url string, startTime, dur time.Duration) (*Downloader, error) {
	vInfo, err := info(url)
	if err != nil {
		return nil, err
	}

	if dur == 0 {
		dur = vInfo.Duration - startTime
	}

	dl := &Downloader{
		Url: url,

		Start:    startTime,
		Duration: dur,
		End:      startTime + dur,

		pause: make(chan wav.State),
		done:  make(chan struct{}, 1),
		pb:    make(chan *wavBundle, preloadCount),
		info:  *vInfo,
	}

	go dl.schedule()

	return dl, nil
}

func (d *Downloader) Stop() {
	d.once.Do(func() {
		close(d.done)
	})
}

func (d *Downloader) Resume() {
	d.pause <- wav.Resume
}

func (d *Downloader) Pause() {
	d.pause <- wav.Pause
}

func (d *Downloader) SendOn(ch chan<- []byte) <-chan struct{} {
	out := make(chan struct{}, 1)

	go func() {
		defer close(out)
		for wavB := range d.pb {
			wavB.wav.Start(ch)

			select {
			case <-d.done:
				wavB.wav.Stop()
				wavB.cleanup()

			case <-wavB.wav.Done:
				break

			case elem := <-d.pause:
				wavB.wav.PlayState <- elem
			}
		}
	}()

	return out
}

func (d *Downloader) schedule() {
	go func() {
		defer close(d.pb)
		for i := 0; ; i++ {
			clipStart := time.Duration(i)*clipTime + d.Start
			clipEnd := time.Duration(i+1)*clipTime + d.Start

			if clipStart >= d.End {
				return
			}

			dur := clipTime
			if clipEnd > d.End {
				dur = d.End - clipStart
			}

			wavb, err := d.download(clipStart, dur)
			if err != nil {
				log.Errorf("error setting up download: %q", err)
				return
			}

			d.pb <- wavb
		}
	}()
}

func (d *Downloader) download(startTime, duration time.Duration) (*wavBundle, error) {
	startSecond := int(startTime.Seconds())
	args := []string{
		"-ss", strconv.Itoa(startSecond),
		"-i", d.info.Url.String(),
		"-c:a", "pcm_s16le",
		"-f", "wav",
		"-ar", "48000",
		"-ac", "2",
		"-vn", "-y",
	}

	dur := int(duration.Seconds())
	if dur > 0 && startTime+duration < d.info.Duration {
		args = append(args, "-t", strconv.Itoa(dur))
	}

	file, err := ioutil.TempFile("", "thulani_")
	if err != nil {
		return nil, err
	}

	clearTemp := func() {
		if err := file.Close(); err != nil {
			log.Errorf("error closing temp file: %q", err)
		}

		if err := os.Remove(file.Name()); err != nil {
			log.Errorf("unable to remove temp file: %q", err)
		}
	}

	args = append(args, file.Name())

	dl := exec.Command(`ffmpeg`, args...)
	b, err := dl.CombinedOutput()
	if err != nil {
		clearTemp()
		log.Errorf("ffmpeg failed: \n%v", string(b))
		return nil, err
	}

	wv, err := wav.New(file.Name())
	if err != nil {
		clearTemp()
		return nil, err
	}

	return &wavBundle{wav: wv, cleanup: clearTemp}, err
}
