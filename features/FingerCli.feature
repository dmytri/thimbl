Feature: Finger command line
  As the operator of thimbl
  I want an explicit command surface
  So that serving is deliberate and setup never binds a socket

  Scenario: Bare thimbl prints usage and does not listen
    When thimbl runs with no subcommand
    Then it exits non-zero
    And the output names the serve command
