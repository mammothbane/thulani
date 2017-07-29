package downloader

import (
	"net/url"
	"os/exec"
	"strconv"
	"time"

	"io/ioutil"
	"os"

	"encoding/json"

	"github.com/mammothbane/thulani-go/wav"
	"github.com/op/go-logging"
)

var log = logging.MustGetLogger("downloader")

// responsible for decoding from youtube
type videoInfo struct {
	Title       string        `json:"fulltitle"`
	UrlStr      string        `json:"url"`
	DurationSec int           `json:"duration"`
	Url         *url.URL      `json:"-"`
	Duration    time.Duration `json:"-"`
}

func info(inUrl string) (*videoInfo, error) {
	dl := exec.Command("youtube-dl", "-f", "bestaudio", "-x", "-j", inUrl)

	outpipe, err := dl.StdoutPipe()
	if err != nil {
		return nil, err
	}

	errpipe, err := dl.StderrPipe()
	if err != nil {
		return nil, err
	}

	err = dl.Start()
	if err != nil {
		log.Errorf("starting youtube-dl failed")
		return nil, err
	}

	o, ierr := ioutil.ReadAll(outpipe)
	if ierr != nil {
		log.Errorf("unable to read from output pipe")
		return nil, err
	}

	e, ierr := ioutil.ReadAll(errpipe)
	if ierr != nil {
		log.Errorf("unable to read from error pipe")
		return nil, err
	}

	if err := dl.Wait(); err != nil {
		log.Errorf("error:\n%v", string(e))
		return nil, err
	}

	v := videoInfo{}
	if err := json.Unmarshal(o, &v); err != nil {
		return nil, err
	}

	v.Duration = time.Duration(v.DurationSec) * time.Second
	v.Url, err = url.Parse(v.UrlStr)

	//tgt, err := url.Parse(string(o))
	//out := tgt.Scheme + "://" + tgt.Host + tgt.Path + "?" + tgt.Query().Encode()
	return &v, err
}

func (d *DownloadManager) download(startTime, duration time.Duration) (<-chan []byte, error) {
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

	ch := make(chan []byte, 1024*32)
	done, err := wav.Load(file.Name(), ch)
	if err != nil {
		clearTemp()
		return nil, err
	}

	go func() {
		<-done
		clearTemp()
	}()

	return ch, err
}
