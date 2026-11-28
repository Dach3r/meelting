module Molecules
  module Brand
    class Component < ViewComponent::Base
      TONES = {
        light: { name: "text-white", brace: "text-lime", dot: "from-lime to-white" },
        dark: { name: "text-ink", brace: "text-forest", dot: "from-lime-deep to-ink" }
      }.freeze

      def initialize(tone: :dark)
        @tone = TONES.fetch(tone)
      end

      private
      attr_reader :tone

      def name_parts
        ApplicationHelper::APP_NAME.downcase.split("i", 2)
      end
    end
  end
end
