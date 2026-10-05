<p align="center">
  <a href="README.ja.md">日本語</a> | <a href="README.zh.md">中文</a> | <a href="README.es.md">Español</a> | <a href="README.fr.md">Français</a> | <a href="README.md">English</a> | <a href="README.it.md">Italiano</a> | <a href="README.pt-BR.md">Português (BR)</a>
</p>

<p align="center"><img src="https://raw.githubusercontent.com/mcp-tool-shop-org/brand/main/logos/runforge/readme.png" alt="RunForge" width="720"></p>

<p align="center"><img src="docs/bench-dark.png" alt="The RunForge window in the dark theme, open on a fixture folder" width="720"></p>

<p align="center">
  <a href="https://github.com/mcp-tool-shop-org/runforge/actions/workflows/ci.yml"><img src="https://github.com/mcp-tool-shop-org/runforge/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue" alt="MIT License"></a>
  <a href="https://mcp-tool-shop-org.github.io/runforge/"><img src="https://img.shields.io/badge/Landing_Page-RunForge-blue" alt="Landing page"></a>
  <a href="https://mcp-tool-shop-org.github.io/runforge/handbook/"><img src="https://img.shields.io/badge/Handbook-RunForge-06b6d4" alt="Handbook"></a>
</p>

# रनफ़ोर्ज

रनफ़ोर्ज, [बैकप्रोपैगेट](https://github.com/mcp-tool-shop-org/backpropagate) आउटपुट फ़ोल्डर के लिए विंडोज़ बेंच है। उस फ़ोल्डर को खोलें जिसमें `run_history.json` है, या `output` निर्देशिका के ऊपर वाला फ़ोल्डर। यह विंडो रन को सूचीबद्ध करती है, संग्रहीत हानि को दर्शाती है, दो पंक्तियों की तुलना करती है, और तालिका या वक्र को निर्यात करती है।

बैकप्रोपैगेट प्रशिक्षक है। इस ऐप में प्रशिक्षक नहीं है, यह कोई मॉडल डाउनलोड नहीं करता है, और न ही यह पायटॉर्च प्रदान करता है। जब `backprop` पहले से ही PATH में है, तो ट्रेन, इवैल और एक्सपोर्ट मॉडल उस कमांड को शुरू करते हैं और उसके लॉग का अनुसरण करते हैं। तर्क ऐप द्वारा बनाए जाते हैं। कुछ भी शेल के माध्यम से नहीं भेजा जाता है। यदि `backprop` गायब है, तो बटन ऐसा बताते हैं। रनफ़ोर्ज बैकप्रोपैगेट डाउनलोड नहीं करता है, इसे स्थापित नहीं करता है, या इसे शामिल नहीं करता है।

वक्र संग्रहीत `loss_history` है, फ़ाइल क्रम में, प्रशिक्षक द्वारा संग्रहीत अधिकतम नमूनों के साथ। `final_loss` एक कॉलम है। इसे पंक्ति में नहीं जोड़ा जाता है। एक शून्य नमूना एक अंतराल है, शून्य नहीं।

## खतरा मॉडल

रनफ़ोर्ज आपके द्वारा चुने गए एक फ़ोल्डर को पढ़ता है। यह उस फ़ोल्डर में `run_history.json` खोलता है, या एक स्तर नीचे `output/run_history.json`, और यह तालिका, वक्र या एक प्रविष्टि को निर्यात कर सकता है। प्राथमिकताएँ, अंतिम फ़ोल्डर और थीम, उस समय पैकेज लोकलस्टेट में लिखी जाती हैं जब ऐप को पैकेज किया जाता है, और निष्पादन योग्य के बगल में जब यह पैकेज नहीं किया जाता है। ट्रेन, इवैल और एक्सपोर्ट मॉडल `backprop` को केवल तभी शुरू करते हैं जब आप बटन दबाते हैं और वह प्रोग्राम पहले से ही PATH में है। लॉग उस प्रोग्राम का आउटपुट है। स्टॉप उस प्रक्रिया ट्री को समाप्त करता है जिसे इस विंडो ने शुरू किया था।

वह डेटा जिसे यह स्पर्श नहीं करता है: प्रशिक्षक, एक मॉडल डाउनलोड, बैकप्रोपैगेट की स्थापना, एक शेल, पर्यावरण की एक प्रति, या टेलीमेट्री। कोई खाता नहीं है।

अनुमतियाँ आपके द्वारा खोले गए फ़ोल्डर, आपके द्वारा चुने गए निर्यात पथ, आपके द्वारा चुने गए डेटा फ़ाइल और प्राथमिकता फ़ाइल पर बनी रहती हैं। ऐप नेटवर्क क्षमता का अनुरोध नहीं करता है।

किसी भेद्यता की रिपोर्ट कैसे करें, यह [SECURITY.md](SECURITY.md) में है।

## बिल्ड

रस्ट 1.98.1, संस्करण 2024। टूलचेन फ़ाइल इसे पिन करती है।

```bash
cargo test --locked --workspace
cargo llvm-cov --locked --workspace --all-targets --lcov --output-path lcov.info --remap-path-prefix --fail-under-lines 90
cargo run -p runforge --locked
```

सीआई कवरेज कमांड चलाता है और `lcov.info` अपलोड करता है। जब पंक्ति कवरेज 90% से कम होता है, तो कोडकोव स्थिति विफल हो जाती है। फ़ाइल संवाद परीक्षणों द्वारा नहीं खोला जाता है।

## स्टोर

प्रकाशित सूची उत्पाद `9PHL1HX0CGMF`, पैकेज `mcp-tool-shop.RunForge-Desktop` है। संस्करण 2 पहले के क्लासिफायर ऐप को बदलता है, और सूची पाठ में उसी सबमिशन में ऐसा उल्लेख होना चाहिए। इस रिपॉजिटरी में अभी तक वह पैकेज नहीं है। जब पैकेज बनाया जाता है तो पैकेज पहचान नहीं बदलती है।

रिकॉर्ड का डिज़ाइन [docs/CONTRACT.md](docs/CONTRACT.md) है। यह रिपॉजिटरी 2.0.0 स्रोत बिल्ड का समर्थन करता है। प्रकाशित स्टोर ऐप 1.0.1 क्लासिफायर बना रहता है जब तक कि `1.0.1.0` से ऊपर का कोई पैकेज सबमिट नहीं किया जाता है।

[एमसीपी टूल शॉप](https://mcp-tool-shop.github.io/) द्वारा निर्मित।
