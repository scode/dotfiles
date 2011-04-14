-- This is my xmonad configuration file. Please do not assume this is
-- idiomatic. I am a newbie to both Haskell and xmonad.
--
-- Requires xmonad-contrib.
--
-- DynamicWorkspaces is the contrib module that gives stumpwm/ion style named
-- workspaces that I cannot live without.

import XMonad
import XMonad.Actions.DynamicWorkspaces
import XMonad.Prompt
import XMonad.Prompt.Shell
import XMonad.Prompt.XMonad
import qualified Data.Map as M

myKeys conf@(XConfig {XMonad.modMask = modm}) =
     [ ((modm, xK_BackSpace), removeWorkspace)
     , ((modm, xK_apostrophe      ), selectWorkspace defaultXPConfig)
     , ((modm, xK_a               ), renameWorkspace defaultXPConfig)
     , ((modm, xK_v               ), spawn $ XMonad.terminal conf)
     , ((modm, xK_c               ), kill)
     ]

newKeys x = M.union (keys defaultConfig x) (M.fromList (myKeys x))

main = xmonad $ defaultConfig
     { modMask = mod4Mask
     , terminal = "/home/scode/bin/xt"
     , keys = newKeys
     }
