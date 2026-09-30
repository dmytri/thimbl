Feature: Finger stdio serving
  As a service supervisor running thimbl from a socket unit
  I want the finger protocol served over stdin and stdout
  So that per-connection socket activation works without thimbl binding any port

  Background:
    Given the user has a project file with content "Deploying thimbl"
    And the user has a plan file with content "Ship the finger server"

  Scenario: A stdio serve answers one query and exits 0
    When thimbl serves stdio with the query "the user's login name"
    Then the stdio answer contains the user's identity line
    And the stdio answer contains the project content "Deploying thimbl"
    And the stdio answer contains the plan content "Ship the finger server"
    And it exits with code 0

  Scenario: A stdio serve prints no startup line
    When thimbl serves stdio with the query "the user's login name"
    Then the stdio answer does not contain "listening on"

  Scenario: An empty stdio query serves the full card
    When thimbl serves stdio with the query ""
    Then the stdio answer contains the user's identity line
    And the stdio answer contains the plan content "Ship the finger server"

  Scenario: A stdio query naming an unknown user gets the no-match answer
    When thimbl serves stdio with the query "ghostuser"
    Then the stdio answer states that the user was not found

  Scenario: A stdio forwarding query is refused
    When thimbl serves stdio with the query "someone@elsewhere.example"
    Then the stdio answer contains "Finger forwarding service denied"

  Scenario: An overlong stdio query is refused
    When thimbl serves stdio with the query of 600 characters
    Then the stdio answer contains "query too long"
    And it exits with code 0
