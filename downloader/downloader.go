package downloader

import (
	"net/url"
	"os/exec"
	"strconv"
	"time"

	"io/ioutil"
	"os"

	"github.com/cryptix/wav"
	"github.com/op/go-logging"
)

var log = logging.MustGetLogger("downloader")

func getUrl(inUrl string) (string, error) {
	dl := exec.Command("youtube-dl", "-f", "bestaudio", "-x", "--get-url", inUrl)

	b, err := dl.CombinedOutput()
	if err != nil {
		log.Errorf("youtube-dl failed: %v", string(b))
		return "", err
	}

	tgt, err := url.Parse(string(b))
	out := tgt.Scheme + "://" + tgt.Host + tgt.Path + "?" + tgt.Query().Encode()

	return out, nil
}

func Download(inUrl string, startTime time.Duration, duration time.Duration) error {
	targetUrl, err := getUrl(inUrl)
	if err != nil {
		return err
	}

	startSecond := int(startTime.Seconds())
	dur := int(duration.Seconds())

	args := []string{
		"-ss", strconv.Itoa(startSecond),
		"-i", targetUrl,
		"-c:a", "pcm_s16le",
		"-f", "wav",
		"-ar", "44100",
		"-ac", "2",
		"-vn", "-y",
	}

	if dur > 0 {
		args = append(args, "-t", strconv.Itoa(dur))
	}

	file, err := ioutil.TempFile("", "dl")
	if err != nil {
		return err
	}
	defer func() {
		if err := os.Remove(file.Name()); err != nil {
			log.Errorf("unable to remove temp file: %q", err)
		}
	}()

	args = append(args, file.Name())

	dl := exec.Command(`ffmpeg`, args...)
	b, err := dl.CombinedOutput()
	if err != nil {
		log.Errorf("ffmpeg failed: \n%v", string(b))
		return err
	}

	info, err := os.Stat(file.Name())
	if err != nil {
		return err
	}

	_, err = wav.NewReader(file, info.Size())
	if err != nil {
		return err
	}

	return nil
}
