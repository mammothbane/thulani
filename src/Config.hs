{-# LANGUAGE DeriveGeneric #-}
{-# LANGUAGE MultiParamTypeClasses #-}

module Config where

import Configuration.Dotenv (defaultConfig, loadFile)
import Control.Monad (void)
import qualified Data.Map as M
import Data.Text
import Discord.Types (Snowflake)
import Env
import Env.Generic
import GHC.Generics
import qualified System.Environment as Env

data Config
  = Config
      { token :: String,
        clientId :: Snowflake,
        databaseUrl :: String,
        ownerId :: Snowflake,
        voiceChannel :: Snowflake,
        maxHist :: Int,
        defaultHist :: Int,
        sheetsApiKey :: String,
        sheetsId :: String
      }
  deriving (Show, Eq, Read, Generic)

instance Field Error Snowflake

instance Record Error Config

loadConfig :: IO Config
loadConfig = do
  void $ loadFile defaultConfig
  Env.parse (header "thulani") (prefixed "THULANI_" record)
