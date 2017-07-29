package downloader

import (
	"time"
)

// DownloadManager handles a download for a particular song.
type DownloadManager struct {
	Url string

	Start    time.Duration
	Duration time.Duration
	End      time.Duration

	pb chan (<-chan []byte)

	info videoInfo
}

const clipTime = 10 * time.Second
const preloadCount = 5

func NewDownload(url string, startTime, dur time.Duration) (*DownloadManager, error) {
	vInfo, err := info(url)
	if err != nil {
		return nil, err
	}

	if dur == 0 {
		dur = vInfo.Duration - startTime
	}

	dl := &DownloadManager{
		Url: url,

		Start:    startTime,
		Duration: dur,
		End:      startTime + dur,

		pb:   make(chan (<-chan []byte), preloadCount),
		info: *vInfo,
	}

	go dl.schedule()

	return dl, nil
}

func (d *DownloadManager) SendOn(ch chan<- []byte) <-chan struct{} {
	out := make(chan struct{}, 1)

	go func() {
		defer close(out)
		for c := range d.pb {
			for b := range c {
				ch <- b
			}
		}
	}()

	return out
}

func (d *DownloadManager) schedule() {
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

			ch, err := d.download(clipStart, dur)
			if err != nil {
				log.Errorf("error setting up download: %q", err)
				return
			}

			d.pb <- ch
		}
	}()
}
