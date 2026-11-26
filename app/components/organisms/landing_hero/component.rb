module Organisms
  module LandingHero
    class Component < ViewComponent::Base
      COMMAND = "docker compose up".freeze

      CHIPS = [
        { key: "read", position: "top-[17%] left-[58%]", accent: false },
        { key: "edit", position: "top-[27%] left-[76%]", accent: true },
        { key: "tests", position: "top-[39%] left-[56%]", accent: false },
        { key: "approval", position: "top-[49%] left-[74%]", accent: true }
      ].freeze

      BUILT_FOR = ["Claude Code", "Codex", "Anthropic", "OpenAI", "Ollama"].freeze

      private

      def command
        COMMAND
      end
    end
  end
end
