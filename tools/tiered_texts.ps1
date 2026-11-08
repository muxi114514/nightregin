<#
  三档锻体与角色专属锻体的文本（由 relic_assets.ps1 引入；参数见 relics\src\tiered\，效果与文本同号）：
    $TieredTexts        锻体名 / 说明（item_dlc01 51 / 52）与状态界面效果名 / 说明（menu_dlc01 214 / 215）
    $TieredStatusTexts  只在状态栏显示的计时效果（只要 menu 的效果名）
    $TieredPassiveTexts 新武器词条的名字 / 说明（item_dlc01 44 / 50，见 relics\src\tiered\passives.rs）
    $TieredAbilityTexts 角色自带能力说明里追加的句子（menu_dlc01 CL_MenuText，From 是原文）
#>
$TieredTexts = @(
    @{ Id = 9971000; Zh = "提升观察力（+20）"; ZhInfo = "观察力提升 20`n可与同名效果叠加"; En = "Improved Item Discovery (+20)"; EnInfo = "Raises item discovery by 20.`nStacks with itself" },
    @{ Id = 9971010; Zh = "减轻黑夜雨的损伤（10%）"; ZhInfo = "被黑夜雨淋到时，受到的损伤减少 10%`n可与同名效果叠加"
       En = "Reduced Night's Tide Damage (10%)"; EnInfo = "Reduces damage taken from the Night's Tide by 10%.`nStacks with itself" },
    @{ Id = 9971100; Zh = "技艺的使用次数＋１"; ZhInfo = "增加 1 次技艺的使用次数"; En = "+1 Character Skill Use"; EnInfo = "Adds one more Character Skill use" },
    @{ Id = 9971110; Zh = "提升观察力（+40）"; ZhInfo = "观察力提升 40`n可与同名效果叠加"; En = "Improved Item Discovery (+40)"; EnInfo = "Raises item discovery by 40.`nStacks with itself" },
    @{ Id = 9971120; Zh = "血量上限减半，攻击命中时恢复血量"; ZhInfo = "血量上限降低 50%`n攻击命中时，恢复 25 点血量`n受到的伤害减少 50%，黑夜雨造成的损伤减少 80%"
       En = "Halved Max HP, HP Restoration on Hit"; EnInfo = "Reduces maximum HP by 50%.`nRestores 25 HP on each successful attack.`nReduces damage taken by 50% and Night's Tide damage by 80%" },
    @{ Id = 9971200; Zh = "死亡时，不会掉落卢恩与降低等级"; ZhInfo = "死亡时，不会掉落持有的卢恩`n也不会降低等级"
       En = "No Rune Loss or Level Down Upon Death"; EnInfo = "Runes are not dropped and level is not lost upon death" },
    @{ Id = 9971300; Zh = "技艺的使用次数＋１（角色专属）"; ZhInfo = "增加 1 次技艺的使用次数`n可与通用的同名锻体叠加"
       En = "+1 Character Skill Use (Hero)"; EnInfo = "Adds one more Character Skill use.`nStacks with the general version" },
    @{ Id = 9971310; Zh = "【铁之眼】延长弱点显露的时间"; ZhInfo = "延长以技艺标记的弱点显露时间"
       En = "[Ironeye] Extends duration of weak point"; EnInfo = "Increases duration of weak point applied by Marking" },
    @{ Id = 9971320; Zh = "【守护者】大幅延长技艺的有效时间"; ZhInfo = "技艺旋风变为持续型（同遗物「延长技艺的有效时间」），`n持续时间再延长一倍，与该遗物同时持有也会生效"
       En = "[Guardian] Greatly Increased Duration for Character Skill"; EnInfo = "Whirlwind lingers as with the relic that increases Character Skill duration,`nand lasts twice as long; also works together with that relic" },
    @{ Id = 9971330; Zh = "【隐士】延长血印的有效时间"; ZhInfo = "延长血魂歌附加的血印有效时间"
       En = "[Recluse] Longer Soulblood mark"; EnInfo = "Increases duration of the blood mark applied by Soulblood Song" },
    @{ Id = 9971340; Zh = "【复仇者】召唤出的家人血量提升 100%"; ZhInfo = "召唤出的家人，血量上限变为两倍"
       En = "[Revenant] Family Max HP +100%"; EnInfo = "Doubles the maximum HP of summoned family" },
    @{ Id = 9971350; Zh = "【执行者】在绝招变身期间锁定满血"; ZhInfo = "在绝招变身期间（化身野兽或熔炉形态），血量始终保持全满`n受到致命伤害也不会死亡"
       En = "[Executor] HP Locked Full while Transformed"; EnInfo = "While transformed (beast or crucible form), HP is kept full.`nLethal damage cannot kill" },
    # 第二批（靠动作脚本触发，见 relics\src\tiered\scripted.rs 与 tools\tiered_buffs.lua）
    @{ Id = 9971130; Zh = "发动绝招后，击杀敌人时返还绝招量表"; ZhInfo = "发动绝招后的 10 秒内击杀敌人时，返还 50% 的绝招量表`n每次发动绝招限 1 次"
       En = "Defeating Enemies after Art Activation Refunds Art Gauge"; EnInfo = "Defeating an enemy within 10 seconds of Art activation refunds 50% of the Art gauge.`nOnce per activation" },
    @{ Id = 9971140; Zh = "圣杯瓶用完时，提升减伤率"; ZhInfo = "圣杯瓶的数量为 0 时，受到的伤害减少 40%"
       En = "Improved Damage Negation with Empty Flasks"; EnInfo = "Reduces damage taken by 40% while out of Flask charges" },
    @{ Id = 9971150; Zh = "发动绝招时，恢复血量"; ZhInfo = "发动绝招时，恢复 50% 的血量"; En = "Art Activation Restores HP"; EnInfo = "Restores 50% of HP upon Art activation" },
    @{ Id = 9971210; Zh = "发动绝招后，绝招量表立即充满"; ZhInfo = "发动绝招后，立刻充满绝招量表`n之后 30 秒内不会再次触发"
       En = "Art Gauge Refills after Art Activation"; EnInfo = "Fully refills the Art gauge right after Art activation.`nCannot trigger again for 30 seconds" },
    @{ Id = 9971220; Zh = "使用圣杯瓶时，不会被打断且不受伤害"; ZhInfo = "饮用圣杯瓶期间，动作不会被打断`n也不会受到伤害"
       En = "Poise and Immunity while Drinking from Flasks"; EnInfo = "While drinking from a Flask, attacks neither interrupt nor damage you" },
    @{ Id = 9971230; Zh = "无法使用绝招，强化技艺"; ZhInfo = "无法再发动绝招`n技艺的冷却时间缩短 50%`n使用技艺后的 10 秒内，攻击力提升 15%，攻击命中时恢复 15 点血量"
       En = "No Ultimate Art, Empowered Character Skill"; EnInfo = "Ultimate Art can no longer be used.`nCharacter Skill cooldown reduced by 50%.`nFor 10 seconds after using the Skill, attack power +15% and each hit restores 15 HP" },
    @{ Id = 9971240; Zh = "无法使用技艺，提升伤害与绝招量表"; ZhInfo = "无法再使用技艺`n造成的伤害提升 30%`n绝招量表的累积速度提升 20%"
       En = "No Character Skill, More Damage and Art Gauge"; EnInfo = "Character Skill can no longer be used.`nDamage dealt +30%.`nArt gauge builds up 20% faster" },
    @{ Id = 9971250; Zh = "共鸣"; ZhInfo = "精准闪避投技，或附近的我方人物发动绝招时，`n能在短时间内免费发动一次绝招（同送葬者的能力）"
       En = "Resonance"; EnInfo = "Perfectly dodging a grab, or a nearby ally activating their Art,`ngrants one free Ultimate Art for a short time (as the Undertaker)" },
    @{ Id = 9971260; Zh = "冷冽海市蜃楼·极"; ZhInfo = "在生死关头时，能隐藏自己的身影，短暂时间内让所有攻击失效`n同时完全恢复血量，30 秒内受到的伤害减少 30%、攻击力提升 15%`n另外持有「冷冽海市蜃楼」时可发动 2 次，在赐福休息时恢复"
       En = "Cold Mirage, Empowered"; EnInfo = "At a critical moment, hide yourself and nullify all attacks for a short time,`nfully restore HP, and for 30 seconds take 30% less damage and deal 15% more.`nWith Cold Mirage also held, it can trigger twice; recovers at a Site of Grace" },
    @{ Id = 9971370; Zh = "【守护者】强化防御时不会耗尽精力"; ZhInfo = "强化防御期间，防御时不消耗精力，精力也不会持续减少"
       En = "[Guardian] Steel Guard Costs No Stamina"; EnInfo = "During Steel Guard, blocking costs no stamina and stamina does not drain" },
    # 第三批（周围异常、武器数量、闪避精力，见 relics\src\tiered\kept.rs）
    @{ Id = 9971160; Zh = "血量全满时，减少闪避消耗的精力"; ZhInfo = "血量全满时，翻滚与后跳消耗的精力减少 25%"
       En = "Reduced Dodge Stamina Cost at Full HP"; EnInfo = "Rolling and backstepping cost 25% less stamina at full HP" },
    @{ Id = 9971170; Zh = "每装备一面盾，提升精力上限"; ZhInfo = "每装备一面盾（小盾、中盾、大盾），精力上限提升 10%"
       En = "Max Stamina Up per Shield Equipped"; EnInfo = "Raises maximum stamina by 10% for each shield equipped (small, medium or great)" },
    @{ Id = 9971180; Zh = "每装备一把法杖或圣印记，提升专注值上限"; ZhInfo = "每装备一把法杖或圣印记，专注值上限提升 10%"
       En = "Max FP Up per Staff or Seal Equipped"; EnInfo = "Raises maximum FP by 10% for each staff or sacred seal equipped" },
    @{ Id = 9971190; Zh = "每装备一面大盾，提升血量上限"; ZhInfo = "每装备一面大盾，血量上限提升 10%"
       En = "Max HP Up per Greatshield Equipped"; EnInfo = "Raises maximum HP by 10% for each greatshield equipped" },
    @{ Id = 9971270; Zh = "周围陷入异常状态时，提升攻击力"; ZhInfo = "周围有人陷入出血、冻伤、中毒、腐败、睡眠、发狂时，`n20 秒内攻击力提升 20%"
       En = "Status Ailments in Vicinity Increase Attack Power"; EnInfo = "When anyone nearby suffers Blood Loss, Frostbite, Poison, Rot, Sleep or Madness,`nattack power +20% for 20 seconds" },
    @{ Id = 9971380; Zh = "【守护者】在能力发动期间，会变得容易受敌人攻击"; ZhInfo = "强化防御期间，会吸引敌人的攻击"
       En = "[Guardian] Draws Enemy Aggression during Steel Guard"; EnInfo = "Steel Guard draws enemy aggression while the stance is active" },
    @{ Id = 9971390; Zh = "【执行者】周围陷入异常状态时，发动能力"; ZhInfo = "周围每有人陷入一次异常状态，就获得一层不屈`n层数没有上限，各层分别计时"
       En = "[Executor] Status Ailments in Vicinity Trigger Tenacity"; EnInfo = "Gain a stack of Tenacity each time anyone nearby suffers a status ailment.`nNo stack limit; each stack expires on its own" },
    @{ Id = 9971400; Zh = "【女爵】周围陷入异常状态时，隐藏身影"; ZhInfo = "周围有人陷入异常状态时，5 秒内身影会变得难以辨识"
       En = "[Duchess] Status Ailments in Vicinity Conceal Self"; EnInfo = "Become concealed for 5 seconds when anyone nearby suffers a status ailment" },
    @{ Id = 9971410; Zh = "【铁之眼】周围陷入异常状态时，提升观察力"; ZhInfo = "周围每有人陷入一次异常状态，观察力永久提升 1"
       En = "[Ironeye] Status Ailments in Vicinity Raise Item Discovery"; EnInfo = "Permanently raises item discovery by 1 each time anyone nearby suffers a status ailment" },
    # 第四批（给附近队友挂效果等，见 relics\src\tiered\aura.rs）
    @{ Id = 9971450; Zh = "【守护者】发动技艺时，自己与周围我方人物获得振奋香"; ZhInfo = "发动技艺时，自己与周围的我方人物获得「振奋香」的效用"
       En = "[Guardian] Skill Grants Uplifting Aromatic to Nearby Allies"; EnInfo = "Using the Character Skill grants the effect of Uplifting Aromatic to self and nearby allies" },
    @{ Id = 9971420; Zh = "【女爵】延长绝招的隐身时间"; ZhInfo = "绝招附加的隐身结束后，再隐身 7.5 秒`n周围被绝招隐身的我方人物也会延长"
       En = "[Duchess] Longer Finale Concealment"; EnInfo = "When Finale's concealment ends, stay concealed for another 7.5 seconds.`nAlso applies to nearby allies concealed by Finale" },
    @{ Id = 9971430; Zh = "【女爵】发动绝招时，提升隐身的我方人物的魔力攻击力"; ZhInfo = "发动绝招时，周围的我方人物在隐身期间，魔力属性攻击力提升 50%"
       En = "[Duchess] Finale Boosts Concealed Allies' Magic Attack"; EnInfo = "On Finale activation, self and nearby allies gain +50% magic attack while concealed" },
    @{ Id = 9971440; Zh = "【女爵】隐身期间，提升造成的伤害"; ZhInfo = "身影难以辨识期间，造成的伤害提升 15%"
       En = "[Duchess] More Damage while Concealed"; EnInfo = "Deal 15% more damage while concealed" },
    @{ Id = 9971460; Zh = "【无赖】发动绝招时，提升周围我方人物的减伤率与攻击力"; ZhInfo = "发动绝招时，自己与周围的我方人物 30 秒内`n受到的伤害减少 15%，攻击力提升 15%"
       En = "[Raider] Totem Stela Protects Nearby Allies"; EnInfo = "On Totem Stela activation, self and nearby allies take 15% less damage`nand gain +15% attack power for 30 seconds" },
    @{ Id = 9971470; Zh = "【无赖】发动绝招时，周围我方人物的动作不会被打断"; ZhInfo = "发动绝招时，自己与周围的我方人物 30 秒内动作不会被打断"
       En = "[Raider] Totem Stela Grants Poise to Nearby Allies"; EnInfo = "On Totem Stela activation, self and nearby allies cannot be staggered for 30 seconds" },
    @{ Id = 9971480; Zh = "【复仇者】延长绝招的不死时间"; ZhInfo = "绝招附加的不死结束后，再延长 7.5 秒`n周围被绝招附加不死的我方人物也会延长"
       En = "[Revenant] Longer Immortal March"; EnInfo = "When Immortal March's immortality ends, it lasts another 7.5 seconds.`nAlso applies to nearby allies" },
    @{ Id = 9971490; Zh = "【复仇者】发动绝招时，我方人物获得一次自行复活"; ZhInfo = "发动绝招后，被绝招附加不死的我方人物，`n倒地时能自行复活一次（1 分钟内有效）"
       En = "[Revenant] Immortal March Grants a Self-Revive"; EnInfo = "After Immortal March, allies granted immortality can revive themselves once when downed.`nLasts 1 minute" },
    @{ Id = 9971496; Zh = "【送葬者】技艺发动期间，动作不会被打断"; ZhInfo = "恍惚期间，动作不会被打断"
       En = "[Undertaker] Poise during Trance"; EnInfo = "Cannot be staggered while in Trance" },
    # 第五批（DLL 实现，见 relics\src\tiered\dll.rs 与 src\tiered_effects）
    @{ Id = 9971020; Zh = "持有的卢恩随机变化"; ZhInfo = "获得时，持有的卢恩随机变为 0.1 ~ 10 倍（上限 100 万）`n每获得一次就生效一次"
       En = "Held Runes Randomly Multiplied"; EnInfo = "On pickup, held runes are multiplied by a random 0.1x ~ 10x (up to 1,000,000).`nTriggers each time it is obtained" },
    @{ Id = 9971195; Zh = "消耗卢恩时，有机会返还"; ZhInfo = "消耗卢恩时，有 40% 的概率全额返还"
       En = "Spent Runes May Be Refunded"; EnInfo = "Spending runes has a 40% chance to refund them in full" },
    @{ Id = 9971280; Zh = "受到致命伤害时，消耗卢恩免于死亡"; ZhInfo = "持有 20000 以上的卢恩时，受到致命伤害会消耗 20000 卢恩，`n免于死亡、完全恢复血量，并对周围的敌人释放冲击波"
       En = "Runes Avert Lethal Damage"; EnInfo = "While holding 20,000+ runes, lethal damage consumes 20,000 runes instead,`nfully restoring HP and releasing a shockwave at nearby foes" },
    @{ Id = 9971290; Zh = "被救起时，补满圣杯瓶"; ZhInfo = "倒地后被我方人物救起时，圣杯瓶的使用次数全部恢复"
       En = "Flasks Refilled When Revived"; EnInfo = "Being revived by an ally fully refills your Flask charges" },
    @{ Id = 9971196; Zh = "圣杯瓶全满时，提升攻击力"; ZhInfo = "圣杯瓶的使用次数为全满时，攻击力提升 15%"
       En = "Attack Up while Flasks Are Full"; EnInfo = "Raises attack power by 15% while your Flask charges are full" },
    @{ Id = 9971030; Zh = "圣杯瓶的使用次数＋１"; ZhInfo = "圣杯瓶的使用次数上限增加 1 次（最多 20 次）`n可与同名效果叠加"
       En = "+1 Flask Charge"; EnInfo = "Raises maximum Flask charges by 1 (up to 20).`nStacks with itself" },
    @{ Id = 9971295; Zh = "每持有一把金色武器，提升造成的伤害"; ZhInfo = "每持有一把金色品质的武器（含以锻造石强化成金色的武器），`n造成的伤害提升 15%"
       En = "Damage Up per Legendary Armament Held"; EnInfo = "Deal 15% more damage for each legendary (gold) armament held,`nincluding ones upgraded with Smithing Stones" },
    @{ Id = 9971198; Zh = "将白色与蓝色武器强化为紫色"; ZhInfo = "获得时，把持有的白色与蓝色武器强化为紫色`n武器的词条也会同锻造石强化一样提升`n每获得一次就生效一次"
       En = "White and Blue Armaments Become Purple"; EnInfo = "On pickup, held common and uncommon armaments are upgraded to rare (purple),`nwith their passives upgraded as with Smithing Stones.`nTriggers each time it is obtained" },
    @{ Id = 9971298; Zh = "提升道具的持有上限，使用道具时效果会扩散到周围的我方人物"; ZhInfo = "消耗品、石剑钥匙与锻造石的持有上限翻倍`n使用道具时，效果也会作用于周围的我方人物"
       En = "Item Capacity Up, Items Affect Nearby Allies"; EnInfo = "Doubles the carry limit of consumables, Stonesword Keys and Smithing Stones.`nItem effects also apply to nearby allies" },
    @{ Id = 9971498; Zh = "【学者】持有的道具上限翻倍"; ZhInfo = "消耗品、石剑钥匙与锻造石的持有上限翻倍`n与「提升道具的持有上限」一起时为 4 倍"
       En = "[Scholar] Item Capacity Doubled"; EnInfo = "Doubles the carry limit of consumables, Stonesword Keys and Smithing Stones.`nWith Item Capacity Up, the limit is 4x" },
    @{ Id = 9971665; Zh = "倒地时，引发剧烈的火焰爆发"; ZhInfo = "倒地时，在原地引发剧烈的火焰爆发，`n对周围的敌人造成火属性伤害"
       En = "Fiery Burst when Downed"; EnInfo = "When downed, unleash a violent burst of flame,`ndealing fire damage to nearby foes" },
    # 血量偏低（DLL，见 relics\src\tiered\low_hp.rs）
    @{ Id = 9971700; Zh = "提升血量偏低时的血量判定"; ZhInfo = "「血量偏低时」类效果的触发线提升 10%（原为 40%）`n可与同名效果叠加，最高 90%"
       En = "Raised Low HP Threshold"; EnInfo = "Raises the HP threshold of low-HP effects by 10% (normally 40%).`nStacks with itself, up to 90%" },
    @{ Id = 9971710; Zh = "血量会被限制在血量偏低的判定线"; ZhInfo = "血量最多只会恢复到「血量偏低时」类效果的触发线以下`n（原为 40%，可被「提升血量偏低时的血量判定」提高）`n血量上限不变"
       En = "HP Held at Low HP Threshold"; EnInfo = "HP cannot recover above the threshold of low-HP effects`n(normally 40%, raised by Raised Low HP Threshold).`nMaximum HP is unchanged" },
    # 第六批（公共事件，见 relics\src\tiered\event.rs 与 tools\tiered_events.ps1）
    @{ Id = 9971670; Zh = "开启监牢时，不再需要石剑钥匙"; ZhInfo = "没有石剑钥匙也能开启监牢，`n开启时不会消耗石剑钥匙"
       En = "Open Gaols without Stonesword Keys"; EnInfo = "Gaols can be opened without a Stonesword Key,`nand opening one does not consume a key" },
    # 第五批角色（动作文件与脚本，见 relics\src\tiered\tae.rs、role_scripts.rs）
    @{ Id = 9971600; Zh = "【铁之眼】技艺会附加猩红腐败，对陷入腐败的敌人造成大伤害"; ZhInfo = "技艺命中时累积猩红腐败，`n对陷入猩红腐败的敌人额外造成大伤害"
       En = "[Ironeye] Skill Inflicts Scarlet Rot, Heavy Damage to Rotting Foes"; EnInfo = "Character Skill hits build up scarlet rot,`ndealing heavy extra damage to foes afflicted by scarlet rot" },
    @{ Id = 9971605; Zh = "【铁之眼】蓄力发动绝招时，附加猩红腐败"; ZhInfo = "蓄力发动绝招时，命中的敌人都会累积猩红腐败"
       En = "[Ironeye] Charged Art Inflicts Scarlet Rot"; EnInfo = "Charged Ultimate Art builds up scarlet rot on every foe it hits" },
    @{ Id = 9971610; Zh = "【学者】以绝招连结的敌人，受到的异常状态伤害大幅提升"; ZhInfo = "以绝招连结的敌人，受到的异常状态伤害提升 75%"
       En = "[Scholar] Linked Foes Take Much More Ailment Damage"; EnInfo = "Foes linked by the Ultimate Art take 75% more damage from status ailments" },
    @{ Id = 9971615; Zh = "【追踪者】非蓄力发动绝招时，返还更多的绝招量表"; ZhInfo = "不蓄力发动绝招时，只消耗 50% 的绝招量表"
       En = "[Wylder] Uncharged Art Refunds More Gauge"; EnInfo = "Uncharged Ultimate Art only consumes 50% of the Art gauge" },
    @{ Id = 9971620; Zh = "【送葬者】强化技艺消耗的绝招量表减少"; ZhInfo = "强化恍惚结束后，返还 45% 的绝招量表`n相当于强化技艺只消耗 55%"
       En = "[Undertaker] Enhanced Trance Costs Less Gauge"; EnInfo = "When Enhanced Trance ends, 45% of the Art gauge is refunded,`nso it effectively costs only 55%" },
    @{ Id = 9971625; Zh = "【女爵】延长专属遗物提供的无敌时间"; ZhInfo = "装备遗物「发动技艺时，能进入短暂无敌状态」时，`n技艺的无敌时间延长为 2 倍"
       En = "[Duchess] Longer Relic Skill Invulnerability"; EnInfo = "With the relic that grants brief invulnerability on Character Skill,`nthe invulnerability lasts twice as long" },
    @{ Id = 9971630; Zh = "【送葬者】延长能力给予的免费绝招时间"; ZhInfo = "能力给予的免费发动绝招的时间延长 3 秒"
       En = "[Undertaker] Longer Bonus Art Window"; EnInfo = "The window to activate a bonus Ultimate Art granted by the ability lasts 3 seconds longer" },
    @{ Id = 9971635; Zh = "【隐士】每吸收一次属性痕，提升属性攻击力"; ZhInfo = "每吸收一次属性痕，魔力、火、雷、圣属性攻击力永久提升 0.5%`n最多 127 次，死亡后也不会消失"
       En = "[Recluse] Each Residue Absorbed Raises Affinity Attack"; EnInfo = "Each affinity residue absorbed permanently raises magic, fire, lightning and holy attack by 0.5%.`nUp to 127 times; kept after death" },
    @{ Id = 9971650; Zh = "【执行者】妖刀攻击时，累积全部异常状态"; ZhInfo = "拔出妖刀期间，每次攻击命中都会累积`n中毒、猩红腐败、出血、冻伤、睡眠与发狂"
       En = "[Executor] Cursed Sword Inflicts All Ailments"; EnInfo = "While the cursed sword is drawn, each hit builds up`npoison, scarlet rot, blood loss, frostbite, sleep and madness" },
    # 第七批角色（改原版判定，见 relics\src\tiered\role_actions.rs）
    @{ Id = 9971740; Zh = "【守护者】战步变为普通翻滚"; ZhInfo = "闪避时不再使出战步，改为普通的翻滚"
       En = "[Guardian] Normal Roll instead of Step"; EnInfo = "Dodging performs a normal roll instead of a step" },
    @{ Id = 9971750; Zh = "【无赖】技艺永远是强化版"; ZhInfo = "不需要在架势中承受攻击，`n技艺必定使出强化后的攻击"
       En = "[Raider] Character Skill Always Enhanced"; EnInfo = "The Character Skill always unleashes its enhanced attack,`nwithout needing to take hits during the stance" },
    @{ Id = 9971755; Zh = "【无赖】重攻击时不会被打断，并提升减伤率"; ZhInfo = "使出重攻击（含蓄力攻击）后的 3 秒内，`n动作不会被投技与抓取以外的攻击打断，受到的伤害减少 30%"
       En = "[Raider] Poise and Damage Negation on Heavy Attacks"; EnInfo = "For 3 seconds after a heavy (including charged) attack,`nactions cannot be interrupted except by throws and grabs, and damage taken is reduced by 30%" },
    @{ Id = 9971760; Zh = "【执行者】延长妖刀完美弹反的判定时间"; ZhInfo = "拔出妖刀防御时，完美弹反（拨开攻击）的判定时间延长 0.15 秒"
       En = "[Executor] Longer Cursed Sword Deflect Window"; EnInfo = "Extends the cursed sword's perfect deflect window by 0.15 seconds" },
    @{ Id = 9971765; Zh = "【执行者】只需完美弹反 1 次，就能解放妖刀"; ZhInfo = "以妖刀完美弹反（拨开攻击）1 次，就能解放刀身"
       En = "[Executor] One Deflect Unleashes the Cursed Sword"; EnInfo = "A single perfect deflect with the cursed sword is enough to unleash the blade" },
    @{ Id = 9971770; Zh = "【执行者】绝招期间，咆哮恢复的血量提升 50%"; ZhInfo = "装备遗物「在绝招发动期间，能以咆哮恢复血量」时，`n咆哮恢复的血量提升 50%（只作用于自己）`n持有【执行者】熔炉妖刀时，熔炉形态下附近友方每秒恢复的血量从 2% 提升到 5%"
       En = "[Executor] Roar Restores 50% More HP"; EnInfo = "With the relic that restores HP by roaring during the Ultimate Art,`nroars restore 50% more HP (self only).`nWith Crucible Cursed Sword, crucible form heals nearby allies 5% HP per second instead of 2%" },
    @{ Id = 9971775; Zh = "【学者】延长技艺弹反屏障的持续时间"; ZhInfo = "对自己使用技艺产生的弹反屏障，持续时间延长 2 秒"
       En = "[Scholar] Longer Parry Barrier"; EnInfo = "The parry barrier from using the Character Skill on yourself lasts 2 seconds longer" },
    # 第八批角色（DLL 改参数与守护者，见 relics\src\tiered\role_params.rs、guardian.rs）
    @{ Id = 9971800; Zh = "【送葬者】延长技艺的持续时间"; ZhInfo = "技艺「恍惚」的持续时间延长 50%（强化版同样延长）"
       En = "[Undertaker] Longer Trance"; EnInfo = "Trance lasts 50% longer (including the enhanced version)" },
    @{ Id = 9971805; Zh = "【学者】观察施加的易伤持续 3 秒，不会立刻消失"; ZhInfo = "技艺观察对敌人施加的 60% 易伤，`n被攻击后仍会持续 3 秒，期间的攻击都能享受易伤"
       En = "[Scholar] Lasting Analysis Weakness"; EnInfo = "The 60% weakness applied by analysis lasts 3 seconds after the first hit,`nso every hit in that window benefits" },
    @{ Id = 9971810; Zh = "【学者】技艺观察完成的速度 +100%"; ZhInfo = "技艺观察敌人的速度提升 100%"
       En = "[Scholar] Faster Analysis"; EnInfo = "Analyzing enemies with the Character Skill is twice as fast" },
    @{ Id = 9971815; Zh = "【复仇者】击杀敌人恢复专注值的概率变为 100%"; ZhInfo = "专属遗物提供的击杀敌人恢复专注值效果，必定发动"
       En = "[Revenant] FP Restoration on Kill Always Triggers"; EnInfo = "The exclusive relic's chance to restore FP on defeating enemies becomes 100%" },
    @{ Id = 9971820; Zh = "【复仇者】家人阵亡后的复活时间 -50%"; ZhInfo = "家人倒下后等待复活期间，血量恢复速度变为两倍`n能更快再次召唤"
       En = "[Revenant] Family Revives 50% Faster"; EnInfo = "Fallen family members recover HP twice as fast while awaiting revival,`nso they can be summoned again sooner" },
    @{ Id = 9971825; Zh = "【守护者】技艺伤害视为战技"; ZhInfo = "技艺旋风（含冰风暴）的伤害也算作战技，`n会受到提升战技攻击力的效果加成"
       En = "[Guardian] Skill Counts as Skill Attack"; EnInfo = "Whirlwind damage (including the ice storm) also counts as a skill attack,`nbenefiting from effects that raise skill attack power" },
    @{ Id = 9971830; Zh = "【守护者】持续释放绝招时，量表消耗减半"; ZhInfo = "绝招落地后持续展开防护时，绝招量表的消耗减少 50%"
       En = "[Guardian] Sustained Ult Drains Half the Gauge"; EnInfo = "While holding the protective aura after the Ultimate Art lands, Art gauge drain is halved" },
    @{ Id = 9971835; Zh = "【守护者】绝招附带圣律，并共享给周围同伴"; ZhInfo = "绝招期间，自己与周围 15 米内的同伴获得「圣律」加成`n（伤害提升 10%，对死诞者伤害提升）"
       En = "[Guardian] Ult Shares Sacred Order"; EnInfo = "During the Ultimate Art, you and allies within 15 m gain Sacred Order`n(10% more damage, extra damage against the undead)" },
    @{ Id = 9971840; Zh = "【守护者】技艺的旋风变为冰风暴"; ZhInfo = "技艺旋风会同时卷起冰风暴，持续时间与旋风相同`n冰风暴持续造成伤害并累积冻伤"
       En = "[Guardian] Whirlwind Becomes Ice Storm"; EnInfo = "The Whirlwind also raises an ice storm lasting as long as the whirlwind,`ndealing continuous damage and building up frostbite" },
    # 第九批（紫10 纯参数；蓝2 / 蓝8 / 蓝14、学者1 由 DLL 改参数，见 relics\src\tiered\buffs.rs、params.rs）
    @{ Id = 9971165; Zh = "提升异常状态的累积（25%）"; ZhInfo = "自己造成的中毒、猩红腐败、冻伤、出血、咒死、睡眠、发狂累积提升 25%`n可与同名效果叠加"
       En = "Increased Status Ailment Buildup (25%)"; EnInfo = "Raises buildup of poison, scarlet rot, frostbite, blood loss, death blight, sleep and madness you inflict by 25%.`nStacks with itself" },
    @{ Id = 9971040; Zh = "护符栏＋１"; ZhInfo = "增加 1 个护符栏位（最多 4 个）`n可与同名效果叠加"
       En = "+1 Talisman Slot"; EnInfo = "Adds one talisman slot (up to 4).`nStacks with itself" },
    @{ Id = 9971050; Zh = "道具栏＋１"; ZhInfo = "增加 1 个道具栏位（最多 8 个）`n可与同名效果叠加"
       En = "+1 Item Slot"; EnInfo = "Adds one item slot (up to 8).`nStacks with itself" },
    @{ Id = 9971060; Zh = "延缓黑夜雨的致命时间（1 分钟）"; ZhInfo = "待在黑夜雨中时，伤害加重前的时间延长 1 分钟`n可与同名效果叠加"
       En = "Night's Tide Turns Lethal Later (1 min)"; EnInfo = "While in the Night's Tide, damage takes 1 more minute to intensify.`nStacks with itself" },
    @{ Id = 9971845; Zh = "【学者】道具进入道具栏后直接满级"; ZhInfo = "道具不需要累积经验，进入道具栏就是最高等级"
       En = "[Scholar] Items Start at Max Level"; EnInfo = "Items need no experience and are at maximum level as soon as they enter your pouch" },
    # 第十批（追踪者12 由 DLL 挂回原版第六感，见 relics\src\tiered\dll.rs）
    @{ Id = 9971850; Zh = "【追踪者】第六感的使用次数＋１"; ZhInfo = "第六感发动后，约 2 秒后还能再发动 1 次`n额外的次数与原版一样，在赐福休息时恢复"
       En = "[Wylder] +1 Sixth Sense Use"; EnInfo = "After Sixth Sense triggers, it becomes ready once more about 2 seconds later.`nThe extra use recovers at a Site of Grace, like the original" },
    @{ Id = 9971070; Zh = "可以在装备界面快速打开锻造台"; ZhInfo = "装备界面右上角会出现「强化武器」「继承词条」按钮，`n点按钮（或按 F3、F4 键）能直接打开对应的界面"
       En = "Open the Forge from the Equipment Menu"; EnInfo = "Reinforce and Inherit Passives buttons appear at the top right of the equipment menu.`nClick them (or press F3 / F4) to open those menus directly" },
    # 第十一批（隐士1 由 DLL 写混合魔法的属性槽，见 src\tiered_effects\direct_cocktail.rs）
    @{ Id = 9971865; Zh = "【执行者】熔炉妖刀"; ZhInfo = "妖刀的重攻击变为奥陶漩涡，蓄力释放强化版`n妖刀的技艺变为纳刀架势：轻攻击释放原来的斩击`n架势重攻击：已解放时释放志留亚漩涡（可蓄力，不消耗解放），未解放时释放百相之羽`n解放后的斩击、奥陶漩涡、志留亚漩涡、百相之羽释放期间处于无敌状态`n绝招改为熔炉形态（不再化身野兽）：换上熔炉斧形套装，妖刀一直拔出，不能使用技艺，可持刀疾跑`n熔炉形态下霸体，减伤 40%，攻击力 +10%，攻击与防御不消耗精力，附近友方每秒恢复 2% 血量；持续时间同原绝招"
       En = "[Executor] Crucible Cursed Sword"; EnInfo = "The cursed sword's heavy attack becomes Ordovis's Vortex, stronger when charged.`nIts skill becomes an unsheathing stance: light attack performs the usual slash.`nStance heavy attack: Siluria's Woe when unleashed (chargeable, does not consume the unleash), Crucible Wings when sealed.`nYou are invincible while performing the unleashed slash, Ordovis's Vortex, Siluria's Woe and Crucible Wings.`nThe Ultimate Art becomes Crucible Form instead of the beast: you don the Crucible Axe set, the cursed sword stays drawn, the skill is disabled and you can sprint with the sword drawn.`nIn Crucible Form you are unflinching, take 40% less damage, deal 10% more damage, attacks and guarding cost no stamina, and nearby allies recover 2% HP per second. Lasts as long as the original Ultimate Art" },
    @{ Id = 9971860; Zh = "【隐士】技艺直接释放混合魔法"; ZhInfo = "技艺不再吸收属性痕，改为直接释放混合魔法，冷却 8 秒；每次释放视为吸收了三个属性痕`n属性取右手三把武器的元素，都没有属性时为三魔力`n法杖算魔力；圣印记：碎石算雷，狩猎神祇、巨人、癫火、龙飨算火，黄金树、黄金律法算圣"
       En = "[Recluse] Skill Casts Magic Cocktails Directly"; EnInfo = "The skill no longer absorbs residues and casts a Magic Cocktail instead, with an 8 second cooldown. Each cast counts as absorbing three residues.`nAffinities come from the three right-hand armaments; with none, the cocktail is pure magic.`nStaves count as magic. Seals: Gravel Stone is lightning; Godslayer's, Giant's, Frenzied Flame and Dragon Communion are fire; Erdtree and Golden Order are holy" }
)
# 会在状态栏显示的计时效果（只要 menu 的效果名）
$TieredStatusTexts = @(
    @{ Id = 9971131; Zh = "击杀时返还绝招量表"; En = "Art Gauge Refund on Kill" },
    @{ Id = 9971141; Zh = "提升减伤率"; En = "Improved Damage Negation" },
    @{ Id = 9971213; Zh = "绝招量表充满（冷却中）"; En = "Art Gauge Refill (Cooldown)" },
    @{ Id = 9971231; Zh = "提升攻击力，攻击命中时恢复血量"; En = "Attack Up, HP Restoration on Hit" },
    @{ Id = 9971258; Zh = "共鸣：能免费发动绝招"; En = "Resonance: Free Ultimate Art" },
    @{ Id = 9971432; Zh = "隐身期间提升魔力攻击力"; En = "Magic Attack Up while Concealed" },
    @{ Id = 9971462; Zh = "提升减伤率与攻击力"; En = "Damage Negation and Attack Up" },
    @{ Id = 9971472; Zh = "动作不会被打断"; En = "Cannot Be Staggered" },
    @{ Id = 9971493; Zh = "倒地时能自行复活一次"; En = "Revive Once When Downed" },
    @{ Id = 9971197; Zh = "圣杯瓶全满，提升攻击力"; En = "Attack Up (Full Flasks)" },
    @{ Id = 9971262; Zh = "冷冽海市蜃楼：减伤与攻击力提升"; En = "Cold Mirage: Negation and Attack Up" }
)
# 执行者【不屈】：妖刀解放状态的属性（relics\src\tiered\tae.rs 的 UNLEASHED_SWORD_STATS）
$TieredAbilityTexts = @(
    @{ Bnd = "menu_dlc01"; Fmg = 200; Id = 416107
       From = @{ zhocn = "活性效果能够提升攻击力与精力恢复速度。"; engus = "Effect boosts attack and stamina recovery speed." }
       zhocn = "活性效果能够提升攻击力与精力恢复速度。`n妖刀解放期间，攻击力提升 15%、减伤率提升 20%，`n但降低对所有异常状态的抵抗力。"
       engus = "Effect boosts attack and stamina recovery speed.`nWhile the cursed sword is unleashed, attack +15% and`ndamage negation +20%, but all resistances are lowered." }
)
$TieredPassiveTexts = @(
    @{ Id = 9972000; Zh = "持有时，发动战技会进入霸体状态"; ZhInfo = "持有这把武器时，发动战技后的 2 秒内`n动作不会被投技与抓取以外的攻击打断"
       En = "Poise when Using Skills While Held"; EnInfo = "While this armament is held, for 2 seconds after using a skill,`nactions cannot be interrupted except by throws and grabs" },
    @{ Id = 9972010; Zh = "持有时，发动魔法、祷告会进入霸体状态"; ZhInfo = "持有这把武器时，咏唱魔法、祷告后的 2 秒内`n动作不会被投技与抓取以外的攻击打断"
       En = "Poise when Casting Spells While Held"; EnInfo = "While this armament is held, for 2 seconds after casting a spell,`nactions cannot be interrupted except by throws and grabs" },
    @{ Id = 9972020; Zh = "持有时，大幅提升仇恨值"; ZhInfo = "持有这面盾时，敌人会优先攻击自己"
       En = "Greatly Draws Enemy Attention While Held"; EnInfo = "While this shield is held, enemies are much more likely to target you" }
)
