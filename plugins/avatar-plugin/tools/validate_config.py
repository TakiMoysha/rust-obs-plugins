#!/usr/bin/env python3
"""
Validator for Bongo Cat avatar configuration files.
Checks JSON syntax and structure validity.
"""

import json
import sys
from pathlib import Path
from typing import List, Dict, Any


class ConfigValidator:
    def __init__(self, base_path: Path):
        self.base_path = base_path
        self.errors: List[str] = []
        self.warnings: List[str] = []
    
    def error(self, msg: str):
        self.errors.append(f"❌ ERROR: {msg}")
    
    def warning(self, msg: str):
        self.warnings.append(f"⚠️  WARNING: {msg}")
    
    def info(self, msg: str):
        print(f"ℹ️  {msg}")
    
    def validate_json(self, file_path: Path) -> Dict[str, Any] | None:
        """Validate JSON syntax and load the file."""
        if not file_path.exists():
            self.error(f"File not found: {file_path}")
            return None
        
        try:
            with open(file_path, 'r', encoding='utf-8') as f:
                return json.load(f)
        except json.JSONDecodeError as e:
            self.error(f"Invalid JSON in {file_path}: {e}")
            return None
    
    def validate_face_config(self) -> bool:
        """Validate face/config.json"""
        self.info("Validating face configuration...")
        
        config_path = self.base_path / "face" / "config.json"
        config = self.validate_json(config_path)
        if not config:
            return False
        
        # Check required fields
        if "HotKey" not in config:
            self.error("face/config.json: Missing 'HotKey' field")
            return False
        
        if "FaceImageName" not in config:
            self.error("face/config.json: Missing 'FaceImageName' field")
            return False
        
        # Check arrays length match
        hot_keys = config["HotKey"]
        images = config["FaceImageName"]
        
        if len(hot_keys) != len(images):
            self.error(f"face/config.json: HotKey count ({len(hot_keys)}) != FaceImageName count ({len(images)})")
            return False
        
        # Check if image files exist
        face_dir = self.base_path / "face"
        for img in images:
            img_path = face_dir / img
            if not img_path.exists():
                self.error(f"Missing face image: {img_path}")
        
        self.info(f"✅ Face config valid: {len(hot_keys)} expressions")
        return True
    
    def validate_mode_list(self) -> List[str]:
        """Validate mode/config.json and return list of modes."""
        self.info("Validating mode list...")
        
        config_path = self.base_path / "mode" / "config.json"
        config = self.validate_json(config_path)
        if not config:
            return []
        
        if "ModelPath" not in config:
            self.error("mode/config.json: Missing 'ModelPath' field")
            return []
        
        modes = config["ModelPath"]
        self.info(f"✅ Found {len(modes)} modes: {', '.join(modes)}")
        return modes
    
    def validate_mode_config(self, mode_name: str) -> bool:
        """Validate individual mode configuration."""
        self.info(f"Validating mode: {mode_name}...")
        
        mode_dir = self.base_path / "mode" / mode_name
        if not mode_dir.exists():
            self.error(f"Mode directory not found: {mode_dir}")
            return False
        
        config_path = mode_dir / "config.json"
        config = self.validate_json(config_path)
        if not config:
            return False
        
        # Required fields
        required = ["BackgroundImageName", "CatBackgroundImageName"]
        for field in required:
            if field not in config:
                self.error(f"{mode_name}/config.json: Missing required field '{field}'")
        
        # Check background images
        for bg_field in ["BackgroundImageName", "CatBackgroundImageName"]:
            if bg_field in config:
                bg_path = mode_dir / config[bg_field]
                if not bg_path.exists():
                    self.error(f"Missing background: {bg_path}")
        
        # Validate hands if configured
        for hand in ["LeftHand", "RightHand"]:
            hand_path_field = f"{hand}ImagePath"
            hand_up_field = f"{hand}UpImageName"
            hand_images_field = f"{hand}ImageName"
            
            if hand_path_field in config and config.get(hand_path_field):
                hand_dir = mode_dir / config[hand_path_field]
                if not hand_dir.exists():
                    self.error(f"Missing {hand} directory: {hand_dir}")
                else:
                    # Check up image
                    if hand_up_field in config:
                        up_img = hand_dir / config[hand_up_field]
                        if not up_img.exists():
                            self.error(f"Missing {hand} up image: {up_img}")
                    
                    # Check animation frames
                    if hand_images_field in config:
                        for img in config[hand_images_field]:
                            img_path = hand_dir / img
                            if not img_path.exists():
                                self.warning(f"Missing {hand} frame: {img_path}")
        
        # Validate keys if configured
        if "KeysImagePath" in config and config.get("KeysImagePath"):
            keys_dir = mode_dir / config["KeysImagePath"]
            if not keys_dir.exists():
                self.error(f"Missing keys directory: {keys_dir}")
            elif "KeysImageName" in config:
                for img in config["KeysImageName"]:
                    img_path = keys_dir / img
                    if not img_path.exists():
                        self.warning(f"Missing key image: {img_path}")
        
        self.info(f"✅ Mode '{mode_name}' config valid")
        return True
    
    def validate_all(self) -> bool:
        """Validate entire avatar configuration."""
        print(f"\n{'='*60}")
        print(f"🔍 Validating Bongo Cat Avatar: {self.base_path}")
        print(f"{'='*60}\n")
        
        # Validate face config
        self.validate_face_config()
        
        # Validate mode list
        modes = self.validate_mode_list()
        
        # Validate each mode
        for mode in modes:
            self.validate_mode_config(mode)
        
        # Print results
        print(f"\n{'='*60}")
        print("📊 Validation Results")
        print(f"{'='*60}\n")
        
        if self.warnings:
            print("⚠️  Warnings:")
            for w in self.warnings:
                print(f"  {w}")
            print()
        
        if self.errors:
            print("❌ Errors:")
            for e in self.errors:
                print(f"  {e}")
            print()
            print(f"❌ Validation FAILED: {len(self.errors)} error(s)\n")
            return False
        else:
            print("✅ Validation PASSED")
            if self.warnings:
                print(f"⚠️  {len(self.warnings)} warning(s)")
            print()
            return True


def main():
    if len(sys.argv) > 1:
        avatar_path = Path(sys.argv[1])
    else:
        avatar_path = Path(__file__).parent
    
    validator = ConfigValidator(avatar_path)
    success = validator.validate_all()
    
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
