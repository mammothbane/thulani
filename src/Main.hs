module Main where

import Config
import Configuration.Dotenv (defaultConfig, loadFile)
import Control.Concurrent (threadDelay)
import Control.Monad (void, when)
import Data.Bifunctor
import qualified Data.List as List
import qualified Data.Map as Map
import Data.Text (Text, isPrefixOf, pack, toLower, unpack)
import qualified Data.Text.IO as TIO
import Discord
import qualified Discord.Requests as R
import Discord.Types

main :: IO ()
main = do
  env <- loadConfig
  let authToken = token env
  userFacingError <-
    runDiscord $
      def
        { discordToken = pack $ "Bot " <> authToken,
          discordOnEvent = handler
        }
  TIO.putStrLn userFacingError

handler :: DiscordHandle -> Event -> IO ()
handler dis (MessageCreate m) = when (shouldHandle m) $ do
  react' "mega"
  reply' "meme"
  where
    react' = send . react m
    reply' = send . reply m
    send :: FromJSON a => R.ChannelRequest a -> IO ()
    send = void . restCall dis
handler _ _ = pure ()

react :: Message -> Text -> R.ChannelRequest ()
react = R.CreateReaction . reactInfo
  where
    reactInfo m = (messageChannel m, messageId m)

reply :: Message -> Text -> R.ChannelRequest Message
reply = R.CreateMessage . messageChannel

(|>) :: a -> (a -> b) -> b
(|>) = flip ($)

shouldHandle :: Message -> Bool
shouldHandle = flip List.all [not . fromBot, isThulaniMessage . messageText] . (|>)

fromBot :: Message -> Bool
fromBot = userIsBot . messageAuthor

isThulaniMessage :: Text -> Bool
isThulaniMessage = flip List.any prefixes . flip isPrefixOf . toLower

prefixes :: [Text]
prefixes =
  [ "!thulani",
    "!thulando",
    "!thulando madando",
    "!todd"
  ]
