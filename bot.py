import discord
import logging
import re
import yaml
from datetime import datetime, timedelta
from contextlib import suppress
from asyncio import Queue, QueueEmpty, sleep, ensure_future, get_event_loop, gather
from urllib.parse import urlsplit
from functools import partial
from youtube_dl.utils import DownloadError

logging.basicConfig(level=logging.INFO)

logger = logging.getLogger('yt-bot')
logger.setLevel(logging.DEBUG)

with open('config.yml') as f:
    config = yaml.load(f)

client = discord.Client()
queue = Queue(maxsize=config.get('queue_size', 0))
current_player = None

reg = re.compile(r'^(?:!|\/){} (.*)$'.format(config['trigger']))


def srv(client):
    sv = config['server']
    if type(sv) is int:
        return discord.utils.get(client.servers, id=sv)
    elif type(sv) is str:
        return discord.utils.get(client.servers, name=sv)
    else:
        raise TypeError('Unknown type {} for server'.format(type(sv)))


@client.event
async def on_ready():
    logger.info('Logged in as {} ({})'.format(client.user.name, client.user.id))


@client.event
async def on_message(message):
    logger.debug('received message \'{}\' from {}#{}:{}, ({})'.format(message.content,
                                                                      message.server,
                                                                      message.channel,
                                                                      message.author.id,
                                                                      'private' if message.channel.is_private else 'public'))

    if message.channel.is_private:
        logger.debug('ignoring private message.')
        return

    comp = message.server.id if type(config['server']) is int else message.server.name

    if comp != config['server']:
        logger.debug('message from wrong server ({})'.format(comp))
        return

    match = reg.search(message.content)
    if not match:
        logger.debug('no match.')
        return

    commands = match.group(1).split()
    command = commands[0]
    author_id = int(message.author.id)

    cmd_map = {
        'skip': stop_player,
        'die': stop_client,
        'sudoku': stop_client,
        'pause': pause,
        'resume': resume,
        'help': partial(print_help, message.channel),
        'list': partial(list_queued, message.channel),
        'queue': partial(list_queued, message.channel),
    }

    if command in cmd_map:
        if author_id == config['admin'] or config['op_role'] in [role.name for role in message.author.roles]:
            logger.info('running command \'{}\''.format(command))
            handle_future(cmd_map[command]())
            return

        logger.info('unauthorized command \'{}\' from member \'{}\' ({})'.format(command, message.author.name, message.author.id))
        handle_future(client.send_message(message.channel, 'fuck you. you\'re not allowed to do that.', tts=message.tts))
        return

    url = urlsplit(command, scheme='https')
    if not (url.netloc and (url.path or (url.path is '/watch' and not url.query))):
        logger.info('syntax error: invalid url \'{}\''.format(command))
        handle_future(client.send_message(message.channel, 'format your commands right. fuck you.', tts=message.tts))
        return

    url = url.geturl()

    if 'imgur' in command:
        logger.info("got imgur link. ignoring.")
        if message.author.name == 'boomshticky':
            handle_future(client.send_message(message.channel, "fuck you conway", tts=True))
        else:
            handle_future(client.send_message(message.channel, "NO IMGUR"))
        return

    logger.debug('playing video from url \'{}\''.format(url))

    handle_future(enqueue_video(url, message))


async def pause():
    global current_player
    if not current_player:
        return

    current_player.pause()
    current_player.acc_time += (datetime.now() - current_player.start_playback_time)
    current_player.start_playback_time = None


async def resume():
    global current_player
    if not current_player:
        return

    current_player.resume()
    current_player.start_playback_time = datetime.now()


async def stop_player():
    global current_player
    if not current_player:
        return

    current_player.stop()
    current_player = None


async def stop_client():
    global current_player
    if not current_player:
        return

    while True:
        try:
            queue.get_nowait()
        except QueueEmpty:
            break

    await stop_player()


async def print_help(channel):
    help_msg = """wew lad. you should know these commands already.

Usage: `!thulani [command]`

commands:
**help**:\t\t\t\tprint this help message
**[url]**:\t\t\t   a url with media that thulani can play. queued up to play after everything that's already waiting.
**list, queue**:\tlist items in the queue, as well as the currently-playing item.
**pause**:\t\t\tpause sound.
**resume**:\t\t resume sound.
**die**:\t\t\t\t empty the queue and stop playing.
**skip**:\t\t\t   skip the current item."""

    handle_future(client.send_message(channel, help_msg))


async def enqueue_video(url, message):
    await connect_voice()

    if not client.is_voice_connected(srv(client)):
        handle_future(client.send_message(message.channel, 'go fuck yourself. voice isn\'t working.', tts=True))
        return

    if queue.full():
        handle_future(client.send_message(message.channel, 'fuck you. wait for the other videos.', tts=True))
        return

    handle_future(queue.put((url, message)))


async def connect_voice():
    if not client.is_voice_connected(srv(client)):
        server = discord.utils.find(lambda x: x.name == config['server'], client.servers)
        voice_chan = discord.utils.find(lambda x: x.name == config['voice_channel'] and x.type is discord.ChannelType.voice, 
                                        server.channels)
        await client.join_voice_channel(voice_chan)


async def list_queued(channel):
    global current_player
    s = ''
    count = 0

    def list_resp(s, count):
        if len(s.strip()) == 0:
            s = 'Queue empty\n'

        slots = config.get('queue_size', 0) 
        if slots is not 0:
            s += '{} slots remaining in the queue.'.format(slots - count)

        handle_future(client.send_message(channel, s.strip()))

    def format_secs(secs):
        durs = ''
        if not secs:
            return durs
        
        if secs > 60:
            durs += '{}m'.format(int(secs / 60))
            durs += '{:02d}s'.format(secs % 60)
        else:
            durs += '{}s'.format(secs % 60)
        return durs
        
    if current_player and not current_player.is_done():
        running_tot = current_player.acc_time
        if current_player.start_playback_time:
            running_tot += (datetime.now() - current_player.start_playback_time)
        
        running = format_secs(int(running_tot.total_seconds()))
        durs = format_secs(current_player.duration)
        
        s += '**{}**: {}{}\n\n'.format('playing' if current_player.is_playing() else 'paused', current_player.title,
                                       '     (*{}*/*{}*)'.format(running, durs) if durs else '')
    
    pairs = []
    while True:
        try:
            pairs.append(queue.get_nowait())

        except QueueEmpty:
            break

    if len(pairs) == 0:
        list_resp(s, count)
        return

    await connect_voice()

    if not client.is_voice_connected(srv(client)):
        client.send_message(channel, 'go fuck yourself. couldn\'t check stored videos', tts=True)
        logger.error('unable to connect to voice!')
        for pair in pairs:
            await queue.put(pair)
        return

    for (url, msg) in pairs:
        if len(msg.embeds) == 0:
            logger.debug('got non-embedded link. creating player to find title.')
            try:
                player = await client.voice_client_in(srv(client)).create_ytdl_player(url)
            except DownloadError:
                logger.exception('unable to download info for {}'.format(url))
            name = player.title
        else:
            name = msg.embeds[0].get('title', None)

        s += '{}\n'.format(name if name and name != '' else '(Unknown)')
        count += 1

        await queue.put((url, msg))

    list_resp(s, count)


die = False

async def run_video():
    global current_player
    vid_logger = logger.getChild('video_scheduler')
    vid_logger.debug('entering run_video')

    while True:
        if die:
            return
        
        try:
            (url, msg) = queue.get_nowait()
        except QueueEmpty:
            await sleep(0.5)
        else:
            break

    try:
        await connect_voice()
    except Exception:
        logger.exception('waiting to connect voice')
        handle_future(run_video())
        return

    if not client.is_voice_connected(srv(client)):
        raise Exception('unable to connect to voice!')
    else:
        vid_logger.info('voice reconnected')

    vid_logger.debug('starting playback')
    try:
        current_player = await client.voice_client_in(srv(client)).create_ytdl_player(url)
    except DownloadError:
        logger.exception('unable to create player for {}'.format(url))
        ensure_future(client.send_message(msg.channel, 'BAD LINK', tts=True))
        handle_future(run_video())
        return

    current_player.start()
    current_player.start_playback_time = datetime.now()
    current_player.acc_time = timedelta()
    vid_logger.debug('playback started')

    has_slept = False
    while True:
        if not current_player or current_player.is_done():
            break

        if not has_slept:
            vid_logger.debug('sleeping')
            has_slept = True

        await sleep(0.5)
    
    if has_slept:
        vid_logger.debug('awoken')

    handle_future(run_video())

def handle_future(fut):
    ensure_future(handle_impl(ensure_future(fut)))

async def handle_impl(fut):
    try:
        await gather(fut)
    except Exception as e:
        logger.exception('awaiting {}'.format(fut))

handle_future(run_video())

loop = get_event_loop()
loop.set_debug(True)

while True:
    if loop.is_closed():
        print('Event loop closed. Exiting.')
        exit()
    
    try:
        logger.info('starting')
        loop.run_until_complete(client.start(config['username'], config['password']))
    except KeyboardInterrupt:
        import time
        die = True

        logger.info('shutting down')
        time.sleep(1.2)

        with suppress(discord.errors.ClientException):
            loop.run_until_complete(client.logout())
            loop.stop()
            loop.close()
    except Exception:
        logger.exception('main loop')
