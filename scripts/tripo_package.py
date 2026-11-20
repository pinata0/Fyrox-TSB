"""Build loose Meshy inputs for 15 characters; retain legacy folder paths."""
from pathlib import Path
from hashlib import sha256
import html
import json
import shutil
import struct
import zlib

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'assets/character-inputs/tripo-roster'
INDIVIDUAL = OUT.parent / 'tripo-individual'
WORK = ROOT / 'artifacts/tripo-animation-inputs'
SPECS = Path(__file__).with_name('tripo_input_specs.json')
CHARACTERS = json.loads(SPECS.read_text(encoding='utf-8'))['characters']

FACE_TARGETS = {
    'eyeBlink': '눈꺼풀을 완전히 닫기 / close eyelids fully without collapsing the eyeball',
    'eyeSquint': '눈 가늘게 뜨기 / narrow the eye opening',
    'eyeWide': '눈 크게 뜨기 / widen the eye opening',
    'browUp': '눈썹 올리기 / raise eyebrow',
    'browDown': '눈썹 내리기 / lower eyebrow',
    'mouthSmile': '입꼬리 올리기 / raise mouth corner',
    'mouthFrown': '입꼬리 내리기 / lower mouth corner',
    'cheekPuff': '볼 부풀리기 / inflate cheek volume',
    'cheekStretch': '볼 옆으로 늘리기 / pull cheek outward laterally',
    'cheekSquash': '볼 누르기·줄이기 / compress cheek inward locally',
}
MORPHS = [dict(name=f'{key}_{side}', range=[0,1], action=action) for key,action in FACE_TARGETS.items() for side in ['L','R']]
MORPHS += [dict(name=name,range=[0,1],action=action) for name,action in [
    ('jawOpen','입 벌리기 / open jaw and mouth cavity'),
    ('mouthPucker','입 오므리기 / pucker lips'),
    ('mouthWide','입 가로로 넓히기 / widen lips without moving cheeks excessively'),
]]

BASE_START = 'Create ONE complete, fully assembled full-body stylized 3D game character from the reference, including the hair, head, body and worn outfit together in the same model. Preserve the cute chibi proportions, oversized head, round cheeks, small body and short limbs.'
BODY_CONNECTION_PROMPT = 'Keep the head, neck, torso, arms, hands, legs and feet continuously connected at their natural anatomical joints. Build a cohesive hairstyle rooted firmly into the scalp: bangs, side locks, braids and ponytails connect through their roots to the head, with no floating hair or gaps at the hairline. Define hair locks with sculpted contours and grooves while preserving their root connections. Any ears, horns, wings or tail shown in the reference attach at their natural roots. Keep clothing fitted on the body. No detached body parts, disassembled character, exploded view or separate hair asset.'
FACE_PROMPT = 'Use open eyes and a relaxed closed mouth. Preserve eye colors and characteristic markings, with natural eyelids, a clear lip line and cheeks continuous with the face.'
NO_NOSE_PROMPT = 'NO NOSE: keep a smooth, gently rounded facial surface between the eyes and mouth. Do not generate a nose, nose bridge, nose tip, nostrils, nose bump, nose line, nose dot, or nose-shaped shading in either geometry or texture. Preserve the eyes, mouth, cheeks and original facial proportions.'
BASE_END = 'Use a neutral A-pose with empty hands and natural clearance between the arms and torso and between the legs, while keeping shoulders, wrists, hips and ankles connected. Hanging hair may clear the limbs and clothing while remaining firmly rooted to the head. Preserve clothing coverage and smooth stylized matte colors. Generate the complete character body with hair and outfit; weapons, mounts and separately supplied props remain separate input jobs. No scenery, text, pedestal or extra characters.'

RIG_COMMON = '''이 파일은 생성 후 구현·검수용 리깅 사양입니다. prompt.txt는 Meshy용 본체 외형 지시이며 Images에서 참고 이미지를 제작·수정할 때 사용합니다.
프롬프트에 요청해도 아래 모프·추가 본이 자동으로 만들어진다고 보장할 수 없습니다.
자동 리깅 결과를 먼저 확인하고 없는 기능은 Blender 등에서 보완합니다.
생성 단계에서는 머리카락·머리·몸·팔다리가 제자리에 연결된 전신 외형을 우선합니다. 머리 뿌리·목·어깨·골반에 틈이 생기지 않는지 확인합니다.
재질별 서브메시나 메시 객체가 여러 개인 것만으로 외형 불량은 아닙니다. 연결부가 떨어져 보이는지와 리깅 시 함께 움직이는지를 검사합니다.

얼굴 / FACE
- 15명 캐릭터 얼굴에는 코·콧대·콧구멍을 만들지 않습니다. 코를 나타내는 점·선·음영도 제외합니다. 세바스티안 인형의 코와 돌고래 물총의 주둥이 등 독립 소품의 고유 형태는 유지합니다.
- 좌우는 화면 기준이 아니라 캐릭터 자신의 해부학적 L/R입니다. 모든 값은 0..1, 0은 중립입니다.
- 눈: Eye.L/R 회전으로 시선 이동. 눈꺼풀은 얼굴의 연속된 메시로 만들고 각 눈을 독립적으로 완전히 닫을 수 있어야 합니다. 홍채/눈동자를 납작한 얼굴 그림으로 고정하지 않습니다.
- 입: 좌우 입꼬리를 독립적으로 올리고 내립니다. 입 안, 치아/작은 송곳니, 필요 시 혀를 별도 메시로 만들고 턱 움직임에 맞춥니다. jawOpen 모프와 턱 본을 중복 적용하지 않습니다.
- 볼: 부풀림, 옆으로 당김, 안으로 눌림을 좌우 각각 만듭니다. 얼굴 전체 크기를 바꾸거나 볼을 별도 떠 있는 구체로 만들지 않습니다. 최대 변형량은 작게 시작해 캐릭터 비율에 맞게 조정합니다.
- 눈썹·속눈썹·블러시가 얼굴과 함께 움직여야 합니다. 안경·가면·왕관 같은 단단한 장식은 볼 모프로 늘리지 않습니다.
- 중립 메시를 정리/리토폴로지한 뒤 동일 정점 수와 순서를 유지하며 shape key를 제작합니다. 표정마다 별도로 생성한 3D 모델은 그대로 모프 타깃으로 쓸 수 없습니다.
- Each listed morph must remain independently controllable. Use additive shapes and corrective shapes for combinations that intersect. No pre-baked single facial expression.

몸·귀·꼬리·머리 / BODY AND SECONDARY MOTION
- Humanoid body rig with pelvis/spine/chest/neck/head, paired arms/hands/fingers and legs/feet. Preserve reference proportions and costume coverage.
- 이름의 {L/R}는 좌우 각각, 01-04는 뿌리에서 끝까지 네 개의 연속 본을 의미합니다. 아래 체인 길이는 제작 권장값이며 자동 생성된 실제 본 목록이 아닙니다.
- 체인 뿌리는 머리/골반/옷의 해당 부위에 고정하고 끝으로 갈수록 부드럽게 변형합니다. 귀와 머리, 꼬리와 치마가 서로 같은 가중치 덩어리가 되지 않게 합니다.
- 동물 귀는 눕힘·세움·작은 떨림, 꼬리는 휘기·흔들기, 머리카락은 회전 시 지연과 복귀를 지원합니다. 뾰족한 사람형 귀는 작은 움직임만 줍니다. 없는 꼬리나 동물 귀를 새로 추가하지 않습니다.
- 머리·어깨·몸통·치마 충돌을 고려합니다. 강성·감쇠·회전 제한·충돌체 등 물리 설정은 게임 엔진에서 별도 구현/조정합니다. 본이 존재하는 것만으로 자동 흔들림이 생기지는 않습니다.
- Hand.R/Hand.L에 Grip.R/Grip.L 소켓을 두고 분리된 무기/소품의 PropRoot를 연결합니다. 손가락과 무기를 한 메시로 용접하지 않습니다. 좌우 손 선택은 원작 조립 참고를 따릅니다.

내보내기·검수 / DELIVERY
- 편집 가능한 .blend 원본, 텍스처, 본/모프 이름 목록을 보관합니다. FBX 또는 GLB는 스킨·모프 보존 여부를 재가져와 확인합니다.
- 본 프로젝트 Fyrox의 해당 파일 형식에서 모프·추가 본을 읽고 구동하는 경로는 이 입력 자료 작업에서 검증하지 않았습니다. 게임 적용 시 임포터와 런타임 연결을 별도로 확인합니다.
- 각 모프를 0/0.5/1로 확인: 눈 완전 닫힘, 입 안 구멍/관통, 볼 변형 중 얼굴 실루엣 유지. blink+smile, jawOpen+cheekStretch 조합도 검사합니다.
- 고개 좌우 회전, 팔 올리기, 귀 눕힘, 꼬리 굽힘, 머리 체인 곡률을 확인하고 소품을 쥔 자세에서 관통을 검사합니다.
- 아래 항목은 제작 요청입니다. 이 패키지에는 완성된 3D 리그나 작동하는 모프 파일이 들어 있지 않습니다.
'''

PROP_RIG = {
 'vivi': 'PropRoot 하나로 단일 수은 칼날을 배치하고 여러 개를 복제해 부유/발사 궤적을 애니메이션합니다. 손잡이를 추가하지 않습니다.',
 'naia': '돌고래 물총은 단단한 PropRoot, 손으로 쥐는 몸통 아래/꼬리 부근에 Grip을 두고 원작 손 자세에 맞춥니다. 주둥이 끝에 WaterMuzzle을 두어 그 위치에서 물줄기 효과를 발사합니다. 돌고래 몸 자체가 물총이므로 별도 권총 손잡이나 총열을 추가하지 않습니다. 작은 꽃 밀짚모자는 분리 액세서리로 두어 표시를 선택할 수 있게 합니다. effect-water는 물총과 별개의 선택 효과 입력이며 입자·투명도·흐름은 엔진에서 제작합니다.',
 'erpin': '지팡이의 손잡이에 Grip 기준점, 전체에 PropRoot. 상단 고리의 빈 공간을 막지 않고 보석은 별도 파츠로 둡니다.',
 'id': '초승달 장치는 MountRoot와 SeatAnchor, 로봇은 RobotRoot와 Ring/Charm 체인으로 별도 제어합니다. 탑승 시 골반을 SeatAnchor에 정렬하되 몸과 장치를 합치지 않습니다.',
 'aya': '양산 손잡이에 Grip, 전체에 ParasolRoot. 필요 시 꽃잎/살대를 개별 파츠와 회전축으로 분리합니다. 펼침/접힘은 별도 모델링·리깅 작업이며 이 한 장으로 정확한 기구가 보장되지는 않습니다.',
 'picora': '별도 생성한 지팡이는 PropRoot와 손잡이 Grip, 카드·별·날개 키링은 Charm.01-03 체인으로 제어합니다. 가방·흰 끈·분홍 리본·체리 장식은 이미 본체 입력에 포함되며 별도 가방을 추가하지 않습니다. 가방은 정면 화면 오른쪽(캐릭터 L) 골반 옆, 흰 끈은 같은 쪽 어깨에 걸린 원래 위치를 유지합니다. 본체 생성 후 가방 부분을 Bag.01에, 끈을 Strap.Front.01-03 / Strap.Back.01-03에 스키닝합니다. 어깨 쪽 끈은 몸통을 따르고 아래쪽 끈은 가방 움직임을 따르게 조정합니다. 분홍 리본은 가방에 고정하고 CherryCharm.01-02는 끈의 체리 장식을 제어합니다. 몸통·치마·팔 관통을 제한합니다. 보조 본과 가중치는 후속 제작 사양이며 구현된 리그가 아닙니다.',
 'chloe': '클로에와 세바스티안은 별도 전신·별도 리그입니다. 클로에에게 곰 머리·곰 귀·인형 팔·보라 망토를 붙이지 않습니다. 세바스티안은 SebastianRoot, Spine/Head, 두 다리, 양쪽 어깨에서 이어지는 Arm.Mint.01-05와 Arm.Pink.01-05, Ear.Mint/Pink, Ribbon.L/R, HeartTuft.01-03, SideOrnament.01-02, Tassel.01-03 체인을 가집니다. 라벤더 하트형 바보털은 곰 머리에, 옆 고리와 술 장식은 분홍 귀 옆에 고정하고 작은 움직임을 허용합니다. 인형 팔 늘림/눌림과 볼 눌림은 인형 자체의 모프로 만듭니다. 양 볼의 하트 선무늬와 퀼팅 선은 변형되는 표면을 따라 움직여야 하며 떠 있는 장식으로 만들지 않습니다. 기본 자세: 클로에 상체/등의 CarryAnchor를 기준으로 세바스티안의 전신을 뒤에 배치하고, 세바스티안 머리의 양 볼 아래에 CheekSupport.L/R 접촉점을 둡니다. 클로에 양팔을 올려 손바닥 IK를 두 접촉점에 맞춥니다. 곰의 보라 리본은 곰 가슴에 남고 클로에 의상이 아닙니다. CarryAnchor와 곰 자세를 먼저 계산한 후 손 IK를 풀어 순환 제약을 피합니다. 곰을 클로에 Head 본에 용접하거나 후드처럼 스킨하지 않습니다. 전투/독립 동작에서도 곰의 팔은 곰의 어깨에 붙어 있습니다. 이 전신에는 양팔이 이미 있으므로 이전 팔 단독 모델을 중복 조립하지 않습니다.',
 'renua': 'MagicCircleRoot 아래 고리별 Ring 본/노드로 회전합니다. 손 앞 EffectAnchor에 배치하며 금색 발광·출현/소멸은 효과 설정으로 구현합니다. 손에 쥐는 총이 아닙니다.',
 'mayo': '발사관 PropRoot와 MouthAim 기준점. 공격 때 입과 관 끝을 정렬하고 손 포즈를 맞춥니다. 양쪽 열린 끝을 유지합니다.',
 'bella': 'FlameRoot로 부유하고 flameStretch/flameSquash는 선택적 효과 모프입니다. 실제 반투명·발광·입자 움직임은 엔진에서 구현합니다.',
 'ui': '클로버 PropRoot/Grip, 잎과 줄기는 작은 굽힘을 허용합니다. 큰 개구리는 FrogRoot, 몸통, 네 다리, 턱/눈꺼풀과 SeatAnchor를 가진 별도 생물 리그로 제작합니다. 인간형 자동 리그를 강제하지 않습니다.',
 'butter': '새총 프레임은 단단한 PropRoot/Grip. 양쪽 고무줄과 중앙 주머니는 분리하고 Band.L/R 및 PouchDraw 또는 bandPull 모프로 당겼다 놓는 동작을 만듭니다. 프레임을 함께 늘리지 않습니다.',
 'komi': '베개 PropRoot와 pillowSquash/pillowStretch 모프, 필요한 경우 가장자리 보조 본을 추가합니다. 프릴은 눌림에 따라 움직이고 손과 베개를 용접하지 않습니다.',
 'hilde': '의료 주사기총은 PropRoot/Grip, 조준/발사 효과 기준점 Muzzle. 약액 창은 별도 재질. 의상이나 손에 합치지 않습니다.',
 'shion': '저격총 PropRoot/Grip, 보조 손 SupportGrip, Muzzle 기준점. 원작 외형을 유지하고 필요할 때만 탄창·지지대를 별도 파츠로 둡니다.',
}

GUIDE = '''Meshy 개별 입력 자료 — 버터 포함 15명 / 2026-11-15

1. 캐릭터별 PNG와 TXT 파일을 직접 사용합니다. Meshy Image to 3D에는 해당 대상의 PNG를 입력합니다. prompt.txt의 코 생략·연결부 지시까지 반영하려면 Images에서 PNG와 프롬프트로 참고 이미지를 수정한 뒤 확인하고 Image to 3D로 보냅니다. 텍스처용 프롬프트만으로 형상이 바뀌지는 않습니다.
2. 본체부터 생성: 캐릭터 PNG에 머리카락·머리·인체·의상을 함께 유지합니다. 머리카락이나 팔다리를 독립 생성하지 않습니다. 무기는 다음 작업에 weapon PNG로 별도 생성합니다. prop-robot / prop-frog / prop-sebastian도 각각 별도 작업입니다. 피코라 가방은 착용한 본체에 포함됩니다. 나이아의 weapon은 돌고래 물총이며 effect-water는 선택적 물줄기 효과입니다.
3. 전신과 무기를 한 화면에 합친 이미지 또는 전체 목록 스크린샷을 입력하지 않습니다. 본체 자동 리깅 뒤 소품은 Blender 등에서 크기·방향·손잡이 위치를 맞추어 조립합니다.
4. rigging.txt와 rig-spec.json은 생성 후 구현·검수 사양입니다. 생성창 텍스트만으로 눈/입/볼 모프와 추가 본이 자동 생성되지는 않습니다. 없는 기능은 후속 작업이 필요합니다.
5. assembly-reference / mounted-reference는 원래 조립 형태를 보는 참고 그림입니다. 분리 본체 작업에는 front.png 또는 개별 캐릭터 PNG를 사용합니다.
6. 클로에는 두 작업입니다: 클로에-chloe.png(클로에 단독)와 클로에-chloe-prop-sebastian.png(곰 인형 전신). 세바스티안은 모자/후드가 아니며 자기 몸통·양팔·다리를 가집니다. 기본 자세는 등 뒤 곰의 볼을 클로에가 양손으로 떠받칩니다. 이전 팔 단독 파일은 이력 폴더로 옮겼으며 전신에 추가 조립하지 않습니다. 세바스티안은 공식 소개 그림의 양 볼 하트 선무늬와 라벤더 하트형 바보털, 보라 리본·옆 고리/술 장식을 사용합니다. 팬 제작 첨부의 질감·노란 막대 형태는 사용하지 않습니다.
7. 피코라는 두 입력만 사용합니다: 피코라-picora.png(가방 착용 본체)와 피코라-picora-weapon.png(지팡이). 기본 본체 이미지에 가방·흰 어깨끈·분홍 리본·체리 장식이 모두 포함됩니다. 가방을 추가로 생성하거나 중복 조립하지 않습니다. 가방 없는 본체, 가방 단독 입력, 중복 조립 참고 그림은 활성 입력 목록에서 제외했습니다.
8. 본체 연결 검수: 앞머리·옆머리·땋은 머리·양갈래의 뿌리는 두피에 붙고, 목·어깨·손목·골반·발목은 이어져 있어야 합니다. 늘어진 머리 끝과 팔다리 사이의 자연스러운 공간은 유지합니다. 메시 객체 수와 실제 연결부의 틈을 구분해 확인합니다.
9. 15명 캐릭터 얼굴에는 코·콧대·콧구멍·코 점·선·음영을 만들지 않습니다. 인형·동물 소품의 원래 코와 주둥이는 유지합니다. 리뉴아 본체는 뒤쪽 원형 판·금색 헤일로·부유 장식을 제거한 새 PNG를 사용하며, 머리 양옆 장식과 별도 공격 마법진은 유지합니다.
10. Split은 파츠 분할 옵션입니다. 볼 모프나 머리카락 본을 자동으로 만드는 기능이 아닙니다. 본체는 A-Pose/T-Pose로 생성하고 리깅을 검사합니다. 필요한 보조 본·모프·물리 동작은 후속 제작합니다.

원작 대표 무기·소품을 기준으로 했습니다. 나이아의 돌고래 물총은 손에 드는 소품이고, 별도 물줄기·도깨비불·마법진은 효과용 형태입니다. 무기 PNG의 weapon이라는 파일명은 공통 분류명입니다.
가려진 후면·두께·연결부, 본체 A포즈는 제작용 해석입니다. 공식 3D 모델 또는 공식 정면도가 아닙니다.
일부 기존 본체 그림의 미소는 외형 참고입니다. 결과의 기준 얼굴은 눈을 뜨고 입을 편하게 다문 중립 상태로 정리한 뒤 표정을 만듭니다.
버터는 중립 얼굴로 다시 생성했습니다. 이드는 흰색/검정 민소매 바디슈트와 금색 눈, 아야는 발목 길이 흰 양갈래를 유지하며 양산을 별도로 생성합니다.

자료 범위: 15개 본체 이미지, {prop_count}개 독립 소품/효과 이미지와 각각의 프롬프트, 15개 리깅 사양. 압축 파일은 만들지 않습니다.
현재 게임의 Blender/FBX와 동작은 이 작업에서 변경하지 않았습니다.
전체 목록: tripo-roster/index.html / 개별 PNG·TXT: tripo-individual/
기존 파일 경로의 tripo 이름은 호환성을 위해 유지하며, 현재 안내와 프롬프트는 Meshy 기준입니다.

공식 기능 확인
https://docs.meshy.ai/en/webapp/image-to-3d
https://help.meshy.ai/en/articles/12005614-how-to-create-images-in-meshy
https://docs.meshy.ai/en/webapp/guides/3d-model/rigging
자동 골격/스키닝과 얼굴 블렌드셰이프·추가 본 제작은 구분해서 검수합니다.
'''
PROP_COUNT = sum(len(c['props']) for c in CHARACTERS)
GUIDE = GUIDE.format(prop_count=PROP_COUNT)

def digest(path):
    return sha256(path.read_bytes()).hexdigest()

def write(path, value):
    path.parent.mkdir(parents=True,exist_ok=True)
    path.write_text(value if isinstance(value,str) else json.dumps(value,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')

def copy_image(source,dest):
    dest.parent.mkdir(parents=True,exist_ok=True)
    shutil.copy2(source,dest)
    data=dest.read_bytes()
    assert data[:8] == b'\x89PNG\r\n\x1a\n', dest
    width,height,depth,color,comp,filt,interlace=struct.unpack('>IIBBBBB',data[16:29])
    assert min(width,height)>0 and (depth,color,comp,filt,interlace)==(8,6,0,0,0), dest
    offset=8; compressed=bytearray()
    while offset<len(data):
        n=struct.unpack('>I',data[offset:offset+4])[0]
        tag=data[offset+4:offset+8]; payload=data[offset+8:offset+8+n]
        assert zlib.crc32(tag+payload)&0xffffffff==struct.unpack('>I',data[offset+8+n:offset+12+n])[0],dest
        if tag==b'IDAT': compressed.extend(payload)
        offset+=n+12
    raw=zlib.decompress(compressed)
    assert len(raw)==height*(width*4+1) and raw[4]==0, dest
    assert digest(source)==digest(dest)
    return dict(file=dest.relative_to(OUT).as_posix(),source=source.relative_to(ROOT).as_posix(),sha256=digest(dest),size=[width,height],mode='RGBA',transparent_corner=True)

def body_prompt(c):
    return '\n\n'.join([BASE_START,c['identity'],c['extra'],BODY_CONNECTION_PROMPT,FACE_PROMPT,NO_NOSE_PROMPT,BASE_END])+'\n'

def prop_prompt(p):
    end='Keep this asset separate from the character. Preserve silhouette, colors, visible thickness, holes and attachment surfaces. Use clean stylized geometry, no hands, person, pedestal, scenery or text. Ignore any surrounding glow in the image; reconstruct only the solid core shape.'
    if p['kind']=='effect': end+=' This is an optional 3D effect proxy; transparency, emission and animated particles will be set up later. Do not convert it into a hand-held gun or lantern.'
    if p['kind']=='deformable_prop': end+=' Use enough continuous surface geometry for later soft bending, stretching and compression while preserving seams.'
    return 'Create ONE separate stylized 3D game asset from the attached reference.\n\n'+p['description']+'\n\n'+end+'\n'

def rig_spec(c):
    spec = dict(schema_version=1,character=c['slug'],status='requested_not_implemented',coordinate_note='L/R are anatomical character sides',face_geometry_note=NO_NOSE_PROMPT,facial_morphs=MORPHS,gaze_bones=['Eye.L','Eye.R'],secondary_chains=c['secondary_bones'],secondary_note=c['motion_note'],prop_rig=PROP_RIG[c['slug']],runtime_status='Fyrox morph import, control and secondary-motion simulation not implemented or verified by this input-pack task')
    if c.get('body_attachment_sockets'):
        spec['body_attachment_sockets'] = c['body_attachment_sockets']
    if c.get('companion_rig'):
        spec['companion_rig'] = c['companion_rig']
    return spec

def rig_text(c):
    rows='\n'.join(f"{m['name']} (0..1): {m['action']}" for m in MORPHS)
    companion='\n동반 인형 독립 리그 / COMPANION RIG\n'+json.dumps(c['companion_rig'],ensure_ascii=False,indent=2)+'\n' if c.get('companion_rig') else ''
    return f"{c['name']} / {c['slug']} — 표정·보조 본·소품 사양\n\n"+RIG_COMMON+'\n필수 얼굴 모프 / REQUIRED MORPHS\n'+rows+'\n\n캐릭터별 보조 본 / SECONDARY CHAINS\n'+c['secondary_bones']+'\n'+c['motion_note']+'\n\n소품 연결·동작 / PROP RIG\n'+PROP_RIG[c['slug']]+'\n'+companion

def catalog(entries):
    esc=html.escape; cards=[]
    for e in entries:
        slug=e['slug']; name=e['name']; sections=[]
        for p in e['jobs']:
            key=slug+'-'+p['key']
            sections.append(f'<section><h3>{esc(p["label"])}</h3><a href="{p["image"]["file"]}"><img loading="lazy" src="{p["image"]["file"]}" alt="{name} {esc(p["label"])}"></a><p>{esc(p["note"])}</p><div class="actions"><a download href="{p["image"]["file"]}">PNG</a><a href="{p["prompt_file"]}">TXT</a><button data-copy="{key}">프롬프트 복사</button></div><textarea readonly id="{key}" aria-label="{name} {esc(p["label"])} 프롬프트">{esc(p["prompt"])}</textarea></section>')
        assembly=f'<p><a href="{e["assembly"]}">조립 참고 그림</a> — 분리 본체 입력용이 아닙니다.</p>' if e.get('assembly') else ''
        if e.get('turnaround'):
            assembly += f'<p><a href="{e["turnaround"]}">착용 삼면도</a> — 측·후면은 제작용 해석이며 단일 본체 생성 입력이 아닙니다.</p>'
        cards.append(f'<article data-search="{name} {slug}"><h2>{name} <small>{slug}</small></h2><p>{esc(e["motion_note"])}</p><div class="actions"><a href="{slug}/rigging.txt">표정·리깅 사양</a><a href="{slug}/rig-spec.json">JSON</a></div>'+assembly+''.join(sections)+'</article>')
    return '''<!doctype html><html lang="ko"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Meshy 입력 · 코 없는 15명 본체와 원작 소품</title><link rel="icon" href="../../ui/icons/favicon.ico" sizes="16x16 32x32 48x48"><style>
*{box-sizing:border-box}body{margin:0;background:#eef2f2;color:#21383c;font:15px/1.6 'Segoe UI','Malgun Gothic',sans-serif}main{max-width:1500px;margin:auto;padding:32px}h1{font-size:32px;line-height:1.3}h2{margin:0}small{font-size:14px;color:#678}header{max-width:1000px}.guide{padding:16px 20px;background:#dcebe6;border-radius:12px}.toolbar,.actions{display:flex;gap:10px;flex-wrap:wrap;align-items:center;margin:16px 0}input{font:inherit;padding:12px;border:1px solid #bdceca;border-radius:8px;width:300px;max-width:100%}a{color:#196657}button,.actions a{font:600 12px 'Segoe UI','Malgun Gothic',sans-serif;padding:9px 12px;background:white;border:1px solid #b4d0c5;border-radius:7px;color:#195f4e;cursor:pointer;text-decoration:none}button{background:#1d705b;color:white}.grid{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:22px;align-items:start}article{background:white;border:1px solid #d7e1e0;border-radius:16px;padding:20px;min-width:0}article p{font-size:13px}section{border-top:1px solid #dce5e2;margin-top:20px}h3{font-size:16px}img{width:100%;height:320px;object-fit:contain;background:#e9edef;border-radius:10px;padding:14px}textarea{width:100%;height:160px;resize:vertical;padding:10px;border:1px solid #cedbd7;border-radius:8px;background:#f7faf9;font:12px/1.5 Consolas,monospace}#status{position:fixed;bottom:20px;left:50%;transform:translateX(-50%);background:#163b31;color:white;padding:12px 18px;border-radius:9px;display:none}.hidden{display:none}@media(max-width:1050px){.grid{grid-template-columns:repeat(2,minmax(0,1fr))}}@media(max-width:680px){main{padding:18px}.grid{grid-template-columns:1fr}}
</style><main><header><small>MESHY INPUTS · 2026-11-15</small><h1>15명 본체 + 원작 무기·소품 '''+str(PROP_COUNT)+'''개</h1><p>PNG·TXT를 직접 사용하세요. Meshy Images에서 참고 PNG와 프롬프트로 이미지를 수정한 뒤 확인하고 Image to 3D로 보냅니다. 기존 PNG를 바로 입력할 수도 있습니다.</p><div class="guide"><b>15명 캐릭터는 코 없이 생성합니다. 무기는 별도로 생성합니다.</b><br>코·콧대·콧구멍·코 점·선·음영을 모두 제외합니다. 인형·동물 소품의 원래 코와 주둥이는 유지합니다. 머리카락·머리·인체는 제자리에 연결된 전신으로 생성합니다.<br>머리 뿌리·목·어깨·골반의 틈을 확인하세요. 늘어진 머리 끝과 팔다리 사이의 자연스러운 공간은 유지합니다.<br>눈·입꼬리·볼 모프와 귀·꼬리·머리카락 보조 본은 후속 작업입니다.<br>리깅 사양은 생성 후 구현·검수용입니다. Meshy 자동 리깅만으로 사용자 지정 모프와 추가 본이 만들어진다고 보장할 수 없습니다.<br>리뉴아는 뒤쪽 헤일로를 제거한 새 본체 이미지를 반영했습니다. 머리 양옆 장식과 별도 공격 마법진은 유지합니다.</div><div class="toolbar"><input id="search" type="search" placeholder="캐릭터 이름 검색" aria-label="캐릭터 검색"><span id="count">15명</span><a href="README.txt">사용방법</a><a href="all-prompts.txt">전체 프롬프트</a></div></header><div class="grid">'''+''.join(cards)+'''</div><p>원작 참고를 바탕으로 한 팬게임 제작용 해석입니다. 각 3D 작업의 결과와 리깅은 별도 검수가 필요합니다.</p></main><div id="status" role="status"></div><script>
function notify(t){const s=document.getElementById('status');s.textContent=t;s.style.display='block';clearTimeout(window.statusTimer);window.statusTimer=setTimeout(()=>s.style.display='none',2200)}
document.addEventListener('click',async e=>{const b=e.target.closest('[data-copy]');if(!b)return;const a=document.getElementById(b.dataset.copy);try{await navigator.clipboard.writeText(a.value);notify('프롬프트를 복사했습니다.')}catch{a.focus();a.select();notify(document.execCommand('copy')?'프롬프트를 복사했습니다.':'Ctrl+C로 복사하세요.')}});
document.getElementById('search').addEventListener('input',e=>{const q=e.target.value.trim().toLowerCase();let n=0;document.querySelectorAll('article').forEach(c=>{const yes=c.dataset.search.toLowerCase().includes(q);c.classList.toggle('hidden',!yes);if(yes)n++});document.getElementById('count').textContent=n+'명'});
</script></html>'''

def main():
    OUT.mkdir(parents=True,exist_ok=True); INDIVIDUAL.mkdir(parents=True,exist_ok=True)
    entries=[]; combined=[]
    for c in CHARACTERS:
        slug=c['slug']; name=c['name']; prefix=f'{name}-{slug}'; folder=OUT/slug
        e=dict(slug=slug,name=name,note=c['note'],motion_note=c['motion_note'],reference_url=c['reference_url'],jobs=[])
        jobs=[dict(key='body',label=c.get('body_label','본체 · 코 없음 · 머리카락과 인체 연결'),source=c['source'],note=c['note'],prompt=body_prompt(c))]
        jobs += [dict(p,prompt=prop_prompt(p)) for p in c['props']]
        for job in jobs:
            key=job['key']; base='front' if key=='body' else key
            prompt_name='prompt.txt' if key=='body' else key+'-prompt.txt'
            info=copy_image(ROOT/job['source'],folder/(base+'.png'))
            write(folder/prompt_name,job['prompt'])
            flat=prefix if key=='body' else prefix+'-'+key
            for src,target in [(folder/(base+'.png'),INDIVIDUAL/(flat+'.png')),(folder/prompt_name,INDIVIDUAL/(flat+'-prompt.txt'))]:
                shutil.copy2(src,target)
            e['jobs'].append(dict(key=key,label=job['label'],kind=job.get('kind','body'),note=job['note'],image=info,prompt_file=f'{slug}/{prompt_name}',prompt=job['prompt']))
            combined.append(f"[{name} / {job['label']}]\n이미지: {slug}/{base}.png\n{job['prompt']}\n")
        for filename,content in [('rigging.txt',rig_text(c)),('rig-spec.json',rig_spec(c))]:
            write(folder/filename,content); dst=INDIVIDUAL/(prefix+'-'+filename); shutil.copy2(folder/filename,dst)
        if c.get('assembly_source'):
            info=copy_image(ROOT/c['assembly_source'],folder/'assembly-reference.png'); e['assembly']=info['file']
            dst=INDIVIDUAL/(prefix+'-assembly-reference.png'); shutil.copy2(folder/'assembly-reference.png',dst)
            if slug in ['id','ui']:
                copy_image(ROOT/c['assembly_source'],folder/'mounted-reference.png')
                write(folder/'mounted-prompt.txt','조립 참고용으로 전환된 자료입니다.\nImage to 3D에는 본체 front.png, 소품은 각 weapon/prop PNG로 별도 생성하세요.\n대응 프롬프트는 참고 이미지 제작·수정 또는 텍스트 입력 지원 모드용 외형 지시입니다.\n표정·리깅은 rigging.txt를 따릅니다.\n')
        if c.get('turnaround_source'):
            # Reference sheets have an opaque background and are not single-view input jobs.
            target=folder/'turnaround-reference.png'
            shutil.copy2(ROOT/c['turnaround_source'],target)
            e['turnaround']=target.relative_to(OUT).as_posix()
            dst=INDIVIDUAL/(prefix+'-turnaround-reference.png')
            shutil.copy2(target,dst)
        mapping='\n'.join(f"{prefix if j['key']=='body' else prefix+'-'+j['key']}.png / 대응 외형 지시: {prefix if j['key']=='body' else prefix+'-'+j['key']}-prompt.txt → {j['label']}" for j in jobs)
        personal=f'{name} 개별 파일 대응\n\n'+mapping+'\n\n'+c['motion_note']+'\n'+PROP_RIG[slug]+'\n\n'+GUIDE
        write(folder/'사용방법.txt',personal); dst=INDIVIDUAL/(prefix+'-사용방법.txt'); shutil.copy2(folder/'사용방법.txt',dst)
        e['individual_files']=[p.relative_to(ROOT).as_posix() for p in sorted(INDIVIDUAL.glob(prefix+'*')) if p.is_file() and p.suffix!='.zip']; entries.append(e)
    expected={c['slug'] for c in json.loads((ROOT/'assets/characters/manifest.json').read_text(encoding='utf-8'))}
    assert len(entries)==15 and {e['slug'] for e in entries}==expected
    assert all(len({p['key'] for p in c['props']})==len(c['props']) for c in CHARACTERS)
    write(OUT/'manifest.json',dict(schema_version=3,target_tool='Meshy',updated='2026-11-15',count=15,prop_count=PROP_COUNT,status='generation_inputs_and_rig_spec_only',entries=entries))
    write(OUT/'all-prompts.txt','\n'.join(combined)); write(OUT/'index.html',catalog(entries)); write(OUT/'README.txt',GUIDE); write(INDIVIDUAL/'사용방법.txt',GUIDE)
    provenance=dict(body_edits=json.loads((WORK/'body-generation-results.json').read_text(encoding='utf-8')),props=json.loads((WORK/'generated-props.json').read_text(encoding='utf-8')),prior_image_generation=json.loads((WORK/'before/tripo-roster/generation-prompts.json').read_text(encoding='utf-8')))
    provenance['reference_edits'] = [json.loads((ROOT/c['reference_generation_record']).read_text(encoding='utf-8')) for c in CHARACTERS if c.get('reference_generation_record')]
    write(OUT/'generation-prompts.json',provenance)
    shutil.copy2(WORK/'research.txt',OUT/'sources.txt')
    print(json.dumps(dict(target_tool='Meshy',characters=15,props=PROP_COUNT,morphs_per_character=len(MORPHS),individual_directory=str(INDIVIDUAL),catalog=str(OUT/'index.html')),ensure_ascii=False))

if __name__=='__main__':
    main()
