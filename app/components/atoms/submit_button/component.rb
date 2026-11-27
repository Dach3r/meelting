module Atoms
  module SubmitButton
    class Component < ViewComponent::Base
      def initialize(**html_options)
        @html_options = html_options
      end

      private
      attr_reader :html_options

      def classes
        class_names(
          "flex w-full cursor-pointer items-center justify-center rounded-full bg-sun-400 px-6 py-5 text-base font-semibold leading-[normal] text-graphite-900 focus:outline-sun-400/40 active:outline-sun-400/40",
          html_options[:class]
        )
      end
    end
  end
end
