-- This is my xmonad configuration file. Please do not assume this is
-- idiomatic. I am a newbie to both Haskell and xmonad.
--
-- Requires xmonad-contrib.
--
-- DynamicWorkspaces is the contrib module that gives stumpwm/ion style named
-- workspaces that I cannot live without.
--
-- TODO: The urgency hook is kinda useless since it only pops up for a while,
-- but better than nothing. I want something that stays up until I visit the
-- window. If you're reading this and know how, please let me know :)

import XMonad
import XMonad.Actions.DynamicWorkspaces
import XMonad.Hooks.ICCCMFocus
import XMonad.Hooks.SetWMName
import XMonad.Hooks.UrgencyHook
import XMonad.Prompt
import XMonad.Prompt.Shell
import XMonad.Prompt.XMonad
import qualified Data.Map as M
import XMonad.Hooks.UrgencyHook

myKeys conf@(XConfig {XMonad.modMask = modm}) =
     [ ((modm, xK_BackSpace), removeWorkspace)
     , ((modm, xK_apostrophe      ), selectWorkspace defaultXPConfig)
     , ((modm, xK_a               ), renameWorkspace defaultXPConfig)
     , ((modm, xK_v               ), spawn $ XMonad.terminal conf)
     , ((modm, xK_c               ), kill)
     ]

newKeys x = M.union (keys defaultConfig x) (M.fromList (myKeys x))

main = xmonad $ withUrgencyHook dzenUrgencyHook { args = [ "-fg", "white", "-bg", "red", "-xs", "1"] }
     $ defaultConfig
     { modMask = mod4Mask
     , terminal = "/usr/local/bin/xt"
     , keys = newKeys
     , startupHook = setWMName "LG3D"
     }
