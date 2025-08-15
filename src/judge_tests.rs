#[cfg(test)]
#[cfg(not(tarpaulin))]
mod judge_tests {
    use std::sync::LazyLock;

    use llm::{builder::LLMBuilder, LLMProvider};

    use crate::check::*;

    static LLM: LazyLock<Box<dyn LLMProvider>> = LazyLock::new(|| {
        LLMBuilder::new()
            .backend(llm::builder::LLMBackend::Ollama) // Use Ollama as the LLM backend
            .base_url(std::env::var("OLLAMA_URL").unwrap_or("http://127.0.0.1:11434".into())) // Set the Ollama server URL
            .model("qwen3:8b")
            .max_tokens(1000) // Set maximum response length
            .temperature(0.7) // Control response randomness (0.0-1.0)
            .stream(false) // Disable streaming responses
            .build()
            .expect("Failed to build LLM (Ollama)")
    });
    static HTTP: LazyLock<reqwest::Client> = LazyLock::new(|| reqwest::Client::new());
    fn e(a: &str, b: &str) -> (String, String) {
        (a.to_string(), b.to_string())
    }
    #[tokio::test]
    async fn test_exact_match() {
        let result = check_correct_answer(
            &LLM,
            &HTTP,
            &"What is the capital of France?",
            "Paris",
            &e("Paris", "Paris"),
            false,
        )
        .await
        .unwrap();

        assert!(matches!(result, Response::Correct), "{:?}", result);
    }

    #[tokio::test]
    async fn test_incorrect_answer() {
        let result = check_correct_answer(
            &LLM,
            &HTTP,
            &"What is the capital of France?",
            "London",
            &e("Paris", "Paris"),
            false,
        )
        .await
        .unwrap();

        assert!(matches!(result, Response::Incorrect(_)), "{:?}", result);
    }

    #[tokio::test]
    async fn test_real_case_1() {
        let result = check_correct_answer(
            &LLM,&HTTP,
            "This quantity is related to a specific wavelength, lambda, by A lambda squared plus B plus C lambda to the minus two plus D lambda to the minus four, where A through D are material constants, in Cauchy's equation. It is sometimes useful to derive this quantity as the square root of relative permittivity times relative permeability. The arcsine of the ratio of this quantity for two media gives the critical angle for (*) total internal reflection. The ratio of this quantity for two media is equal to the ratio of the sine",
            "indxe fo refarction",
            &e("index of refraction [or n until it is read]", "index of <b>refraction</b> [or n until it is read]"),
            false,
        )
        .await
        .unwrap();

        assert!(matches!(result, Response::Correct), "{:?}", result);
    }

    // leniency
    #[tokio::test]
    async fn test_real_case_2() {
        let result = check_correct_answer(
            &LLM,&HTTP,
            r#"The energy eigenspectrum associated with this system's quantum analogue can be solved for analytically using Hermite Polynomials or algebraically using the creation and annihilation operators. If its potential is truncated quadratically in the Taylor series centered around the minimum potential, any arbitrary system can be (*) modelled by this system. The general homogeneous solutions to this system's equations of motion are complex exponentials in time. Approximating sine of x to first order allows for the use of this system for ideal pendulums at small angles. For 10 points, name this physical system which can be used to model frictionless, Hookean springs."#,
            "simple harmonic system",
            &e(r#"simple harmonic oscillators (accept SHOs, prompt on "harmonic oscillators")"#, r#"simple harmonic oscillators (accept SHOs, prompt on "harmonic oscillators")"#),
            false,
        )
        .await
        .unwrap();

        assert!(matches!(result, Response::Incorrect(_)), "{:?}", result);
    }

    // underlined part
    #[tokio::test]
    async fn test_real_case_3() {
        let result = check_correct_answer(
            &LLM,&HTTP,
            r#"Mark Moseley was playing for this team when he became the only placekicker to be awarded MVP. This team reached Super Bowl VII ["seven"] with a team of veterans nicknamed the "Over the Hill Gang". Gary Clark and Ricky Sanders joined a member of "The Fun Bunch", Art Monk, in a wide receiver trio for this team nicknamed "The (*) Posse". Cornerback Darrell Green played his entire career for this team. In the 2016 playoffs, this winner of the NFC East lost to the Green Bay Packers at their home stadium of FedExField. For 10 points, name this NFL team whose name combines a controversial slang term for Native Americans with the US capital."#,
            "redskins",
            &e(r#"<b><u>Washington</u></b> <b><u>Redskins</u></b> [accept either underlined part]"#, r#"Washington Redskins [accept either underlined part]"#),
            false,
        )
        .await
        .unwrap();

        assert!(matches!(result, Response::Correct), "{:?}", result);
    }

    #[tokio::test]
    async fn test_real_case_4() {
        let result = check_correct_answer(
            &LLM,&HTTP,
            r#"Description acceptable. A parody of this event involving the delivery of an old lady's birthday cake was included in the Family Guy episode "Saving Private Brian." A participant in this event said to another, "If you want my shirt, I will give it to you afterwards" in response to unwanted physical contact. Luis Medina Cantalejo witnessed this event and informed Horacio Elizondo of its occurrence. This event's target, who was accused of calling its perpetrator "the son of a (*) terrorist whore," later revealed that his actual words were "I prefer the whore that is your sister." That target was Italian defender Marco Materazzi. For 10 points, identify this event that resulted in the ejection of an illustrious French midfielder from the 2006 World Cup final."#,
            "Headbutt",
            &e(r#"Zinedine <b><u>Zidane headbutt</u></b>ing Marco Materazzi in the 2006 FIFA World Cup Final [or: Zinedine <b><u>Zidane's ejection</u></b>, obvious equivalents; prompt on: "<b><u>2006</u></b> FIFA <b><u>World Cup Final</u></b>", "<b><u>headbutt</u></b>"]"#, r#"Zinedine Zidane headbutting Marco Materazzi in the 2006 FIFA World Cup Final [or: Zinedine Zidane's ejection, obvious equivalents; prompt on: "2006 FIFA World Cup Final", "headbutt"]"#),
            false,
        )
        .await
        .unwrap();

        assert!(matches!(result, Response::Correct), "{:?}", result);
    }
    #[tokio::test]
    async fn test_real_case_5() {
        let result = check_correct_answer(
            &LLM,&HTTP,
            r#"Note to players: The answer to this tossup includes both a phenomenon and a setting, such as "bubbles in water." In one diagram, thirteen classes of these phenomena in this setting are bounded by lines on which the Stix elements S, R, and L are either zero or infinite. Stringer diagrams describe the temperature dependence of these phenomena, expanding on the "cold" set of them found on a CMA diagram. A set of these phenomena that are produced by tension in magnetic field lines travel at a speed proportional to the B-field. Particles with a similar velocity to"#,
            "Radiation",
            &e(r#"<b><u>wave</u></b>s in <b><u>plasma</u></b>s [accept plasma waves; accept <b><u>oscillations</u></b> in <b><u>plasma</u></b>s or <b><u>plasma oscillation</u></b>s before “oscillations”; accept plasma modes; prompt on waves or oscillations or modes or Alfvén waves or Langmuir waves by asking "In what setting?"]"#, r#"waves in plasmas [accept plasma waves; accept oscillations in plasmas or plasma oscillations before “oscillations”; accept <b><u>plasma modes</u></b>; prompt on <u>wave</u>s or <u>oscillation</u>s or <u>mode</u>s or <u>Alfvén wave</u>s or <u>Langmuir wave</u>s by asking "In what setting?"]"#),
            false,
        )
        .await
        .unwrap();

        assert!(matches!(result, Response::Incorrect(_)), "{:?}", result);
    }
    #[tokio::test]
    async fn test_real_case_6() {
        let result = check_correct_answer(
            &LLM,&HTTP,
            r#"The ENLIL model uses the predictions of a model of this phenomenon developed by Wang, Sheeley, and Arge that correlates the speed of this phenomenon with flux tube expansion. A highly variable component of this phenomenon is characterized by a relatively high abundance of elements like magnesium, silicon, and iron that have an FIP (F-I-P) below 10eV (ten-E-V). The development of a 3D time-dependent model of this phenomenon from data recorded by the IMPACT and PLASTIC instruments was a scientific objective of the (+) STEREO mission. Eugene Parker showed that this phenomenon causes a related structure to form a ballerina skirt-like spiral. This phenomenon's 50 year low was observed in 2008 by the spacecraft Ulysses. One component of this phenomenon appears to originate from the helmet (*) streamer belt. In 2018, Voyager II (two) passed out of this phenomenon into the VLISM. This phenomenon changes the direction of a comet's ion tail. Joan Feynman studied how this phenomenon interacts with the magnetosphere to cause auroras. For 10 points, name this plasma formed by charged particles escaping the Sun."#,
            "solar flares",
            &e(r#"<b><u>solar wind</u></b> [or slow <b><u>solar wind</u></b> or fast <b><u>solar wind</u></b>]"#, r#"solar wind [or slow solar wind or fast solar wind]"#),
            false,
        )
        .await
        .unwrap();

        // incorrect or prompted
        assert!(!matches!(result, Response::Correct), "{:?}", result);
    }
    #[tokio::test]
    async fn test_real_case_7() {
        let result = check_correct_answer(
            &LLM,&HTTP,
            r#"This construct can exist if mirror matter exists, and some versions of in include the Somluchowski Trapdoor and the Ranque-Hilsch vortex tube. Landauer and Bennett showed that this construct would have to eventually erase the data that it had collected, and in a criticism of the formulation of this, Leo Szilard noted that taking a measurement would actually require expending energy. Classically, the relative difference in temperature between both parts of this device would increase, and the overall entropy would decrease. For 10 points identify this violator of the second law of thermodynamics who is able to separate"#,
            "Maxwell",
            &e(r#"<b><u>Maxwell's Demon</u></b>"#, r#"Maxwell's Demon"#),
            false,
        )
        .await
        .unwrap();

        assert!(matches!(result, Response::Incorrect(_)), "{:?}", result);
    }
    // #[tokio::test]
    // async fn test_real_case_8() {
    //     let result = check_correct_answer(
    //         &LLM,&HTTP,
    //         r#"Users are given complete control over algorithms for this task through its namesake "activations." Shortly after landing, the Mars Pathfinder needed to be debugged from Earth due to an error in this task, during which a watchdog would periodically trigger system resets. The degree of flexibility allowed by algorithms for this task differentiates hard, firm, and soft real-time systems. In 2023, the EEVDF algorithm replaced the WFQ algorithm used for this task by a namesake (*) "Completely Fair" process in the Linux kernel. Operating systems may leverage run queues and multilevel feedback queues to achieve this task. Basic algorithms for this task include"#,
    //         "Maxwell",
    //         &e(r#"<b><u>Maxwell's Demon</u></b>"#, r#"Maxwell's Demon"#),
    //         false,
    //     )
    //     .await
    //     .unwrap();

    //     assert!(matches!(result, Response::Incorrect(_)), "{:?}", result);
    // }
}
