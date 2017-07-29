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

// downloader handles a download for a particular song.
type downloader struct {
	Url string

	StartTime time.Duration
	Duration  time.Duration
	EndTime   time.Duration

	once sync.Once
	done chan struct{}
	pb   chan *wavBundle

	info videoInfo
}

const clipTime = 10 * time.Second
const preloadCount = 5

func newDownload(url string, startTime, dur time.Duration) (*downloader, error) {
	vInfo, err := info(url)
	if err != nil {
		return nil, err
	}

	if dur == 0 {
		dur = vInfo.Duration - startTime
	}

	dl := &downloader{
		Url: url,

		StartTime: startTime,
		Duration:  dur,
		EndTime:   startTime + dur,

		done: make(chan struct{}, 1),
		pb:   make(chan *wavBundle, preloadCount),
		info: *vInfo,
	}

	go dl.schedule()

	return dl, nil
}

func (d *downloader) Stop() {
	d.once.Do(func() {
		close(d.done)
	})
}

func (d *downloader) Start() (<-chan []byte, <-chan struct{}) {
	out := make(chan []byte, 1024)
	done := make(chan struct{}, 1)

	go func() {
		defer close(done)
		defer close(out)
		for wavB := range d.pb {
			wavB.wav.Start(out)

			select {
			case <-d.done:
				wavB.wav.Stop()
				wavB.cleanup()

			case <-wavB.wav.Done:
				break
			}
		}
	}()

	return out, done
}

func (d *downloader) schedule() {
	defer close(d.pb)
	for i := 0; ; i++ {
		clipStart := time.Duration(i)*clipTime + d.StartTime
		clipEnd := time.Duration(i+1)*clipTime + d.StartTime

		if clipStart >= d.EndTime {
			return
		}

		dur := clipTime
		if clipEnd > d.EndTime {
			dur = d.EndTime - clipStart
		}

		wavb, err := d.downloadSegment(clipStart, dur)
		if err != nil {
			log.Errorf("error setting up download: %q", err)
			return
		}

		d.pb <- wavb
	}
}

func (d *downloader) downloadSegment(startTime, duration time.Duration) (*wavBundle, error) {
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
