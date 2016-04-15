import discord
import logging
import re
import yaml
from asyncio import Queue, QueueEmpty, sleep
from urllib.parse import urlsplit
from functools import partial

logging.basicConfig(level=logging.INFO)

logger = logging.getLogger('yt-bot')
logger.setLevel(logging.DEBUG)

with open('config.yml') as f:
    config = yaml.load(f)

client = discord.Client()
queue = Queue(maxsize=config.get('queue_size', 0))
current_player = None

reg = re.compile(r'^(?:!|\/){} (.*)$'.format(config['trigger']))


@client.event
async def on_ready():
    logger.info('Logged in as {} ({})'.format(client.user.name, client.user.id))


@client.event
async def on_message(message):
    logger.debug('received message \'{}\' from {}#{}, ({})'.format(message.content,
                                                                   message.server,
                                                                   message.channel,
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
        'list': list_queued,
        'queue': list_queued,
    }

    if command in cmd_map:
        if author_id == config['admin'] or config['op_role'] in [role.name for role in message.author.roles]:
            logger.info('running command \'{}\''.format(command))
            cmd_map[command]()
            return

        logger.info('unauthorized command \'{}\' from member \'{}\' ({})'.format(command, message.author.name, message.author.id))
        client.send_message(message.channel, 'fuck you. you\'re not allowed to do that.', tts=message.tts)
        return

    url = urlsplit(command, scheme='https')
    if not (url.netloc and (url.path or (url.path is '/watch' and not url.query))):
        logger.info('syntax error: invalid url \'{}\''.format(command))
        client.send_message(message.channel, 'format your commands right. fuck you.', tts=message.tts)
        return

    url = url.geturl()
    logger.debug('playing video from url \'{}\''.format(url))

    enqueue_video((url, message))


async def pause():
    if not current_player:
        return

    current_player.pause()


async def resume():
    if not current_player:
        return

    current_player.resume()


async def stop_player():
    if not current_player:
        return

    current_player.stop()
    current_player = None


async def stop_client():
    if not current_player:
        return

    while True:
        try:
            queue.get_nowait()
        except QueueEmpty:
            break

    await stop_player()


async def enqueue_video(pair):
    await connect_voice()

    if not client.is_voice_connected():
        client.send_message(pair[1].channel, 'go fuck yourself. voice isn\'t working.', tts=True)
        return

    if queue.full():
        client.send_message(pair[1].channel, 'fuck you. wait for the other videos.', tts=True)
        return

    queue.put(pair)


async def connect_voice():
    if not client.is_voice_connected():
        server = discord.utils.find(lambda x: x.name == config['server'], client.servers)
        voice_chan = discord.utils.find(lambda x: x.name == config['voice_channel'] and x.type is discord.ChannelType.voice, 
                                        server.channels)
        await client.join_voice_channel(voice_chan)


async def list_queued(channel):
    s = ''
    count = 0

    def list_resp(s, count):
        if len(s.strip()) == 0:
            s = 'Queue empty\n'

        slots = config.get('queue_size', 0) 
        if slots is not 0:
            s += '{} slots remaining in the queue.'.format(slots - count)

        client.send_message(channel, s.strip())


    if current_player and not current_player.is_done():
        s += '**{}**: {}\n\n'.format('playing' if current_player.is_playing() else 'paused', current_player.title)
    
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

    if not client.is_voice_connected():
        client.send_message(channel, 'go fuck yourself. couldn\'t check stored videos', tts=True)
        logger.error('unable to connect to voice!')
        for pair in pairs:
            await queue.put(pair)
        return

    for (url, msg) in pairs:
        if len(msg.embeds) == 0:
            logger.debug('got non-embedded link. creating player to find title.')
            player = await client.voice.create_ytdl_player(url)
            name = player.title
        else:
            name = msg.embeds[0].get('title', None)

        s += '{}\n'.format(name if name and name != '' else '(Unknown)')
        count += 1

        await queue.put((url, msg))

    list_resp(s, count)


async def run_video():
    vid_logger = logger.getChild('video_scheduler')
    vid_logger.debug('entering run_video')

    (url, _) = await queue.get()
    await connect_voice()

    if not client.is_voice_connected():
        raise Exception('unable to connect to voice!')
    else:
        vid_logger.info('voice reconnected')

    vid_logger.debug('starting playback')
    current_player = await client.voice.create_ytdl_player(url)
    current_player.start()
    vid_logger.debug('playback started')

    has_slept = False
    while True:
        if not current_player or current_player.is_done():
            break

        if not has_slept:
            vid_logger.debug('sleeping')
            has_slept = True

        await sleep(2)
    
    if has_slept:
        vid_logger.debug('awoken')

        run_video()


run_video()
client.run(config['username'], config['password'])
